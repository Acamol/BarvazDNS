use std::sync::OnceLock;

use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::sync::oneshot;

use crate::common::config::Config;
use crate::common::consts::WEB_DASHBOARD_PORT;
use crate::common::message::{Request, Response};
use crate::common::strings::VERSION;

const DASHBOARD_HTML: &str = include_str!("dashboard.html");
const DASHBOARD_CSS: &str = include_str!("style.css");
const DASHBOARD_JS: &str = include_str!("dashboard.js");

static SHUTDOWN_TX: OnceLock<tokio::sync::Mutex<Option<oneshot::Sender<()>>>> = OnceLock::new();

fn read_port_from_config() -> u16 {
    Config::get_config_file_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| toml::from_str::<Config>(&s).ok())
        .map(|c| c.effective_dashboard_port())
        .unwrap_or(WEB_DASHBOARD_PORT)
}

pub fn router() -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/style.css", get(stylesheet))
        .route("/dashboard.js", get(script))
        .route("/api/status", get(api_status))
        .route("/api/config", get(api_config))
        .route("/api/update", post(api_force_update))
        .route("/api/reload", post(api_reload))
}

pub async fn start() {
    let _ = SHUTDOWN_TX.get_or_init(|| tokio::sync::Mutex::new(None));

    loop {
        let port = read_port_from_config();
        let app = router();
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to bind web server on {addr}: {e}");
                return;
            }
        };
        log::info!("Web dashboard listening on http://{addr}");

        let (tx, rx) = oneshot::channel::<()>();
        if let Some(lock) = SHUTDOWN_TX.get() {
            *lock.lock().await = Some(tx);
        }

        if let Err(e) = axum::serve(listener, app)
            .with_graceful_shutdown(async {
                rx.await.ok();
            })
            .await
        {
            log::error!("Web server error: {e}");
            return;
        }

        log::info!("Web dashboard reloading...");
    }
}

async fn dashboard() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        DASHBOARD_HTML,
    )
}

async fn stylesheet() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        DASHBOARD_CSS,
    )
}

async fn script() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            header::CONTENT_TYPE,
            "application/javascript; charset=utf-8",
        )],
        DASHBOARD_JS,
    )
}

async fn api_status() -> impl IntoResponse {
    match Request::GetStatus.send().await {
        Ok(Response::Status(status)) => {
            let to_millis = |t: std::time::SystemTime| {
                t.duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            };
            let (last_update, updated_domains) = match &status.last_success {
                Some((t, domains)) => (Some(to_millis(*t)), Some(domains.clone())),
                None => (None, None),
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "last_update": last_update,
                    "updated_domains": updated_domains,
                })),
            )
        }
        Ok(Response::Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "unexpected response" })),
        ),
    }
}

async fn api_config() -> impl IntoResponse {
    match Request::GetConfig.send().await {
        Ok(Response::Config(config)) => {
            let domains: Vec<&String> = config.domain.iter().collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "version": VERSION,
                    "interval": humantime::format_duration(config.interval).to_string(),
                    "ipv6": config.ipv6 == Some(true),
                    "token_set": config.token.is_some(),
                    "domains": domains,
                })),
            )
        }
        Ok(Response::Err(e)) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": e })),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({ "error": e.to_string() })),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": "unexpected response" })),
        ),
    }
}

async fn api_force_update() -> impl IntoResponse {
    match Request::ForceUpdate.send().await {
        Ok(Response::Ok) => Json(serde_json::json!({ "ok": true })),
        Ok(Response::Err(e)) => Json(serde_json::json!({ "ok": false, "error": e })),
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e.to_string() })),
        _ => Json(serde_json::json!({ "ok": false, "error": "unexpected response" })),
    }
}

async fn api_reload() -> impl IntoResponse {
    if let Some(lock) = SHUTDOWN_TX.get()
        && let Some(tx) = lock.lock().await.take()
    {
        let _ = tx.send(());
        return Json(serde_json::json!({ "ok": true }));
    }
    Json(serde_json::json!({ "ok": false, "error": "no active server" }))
}
