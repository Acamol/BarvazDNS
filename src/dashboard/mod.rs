use std::sync::OnceLock;

use axum::http::{StatusCode, header};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::sync::oneshot;

use crate::common::config::Config;
use crate::common::consts::WEB_DASHBOARD_PORT;
use crate::common::message::{Request, Response};
use crate::common::strings::{
    AUTHORS, DESCRIPTION, LICENSE, LOG_FILE_BASENAME, REPOSITORY, VERSION,
};
use crate::common::version_check;

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
        .route("/api/check-update", get(api_check_update))
        .route("/api/do-update", post(api_do_update))
        .route("/api/logs", get(api_logs))
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
                    "authors": AUTHORS,
                    "description": DESCRIPTION,
                    "license": LICENSE,
                    "repository": REPOSITORY,
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

async fn api_check_update() -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(version_check::check_for_update).await;
    match result {
        Ok(Some(tag)) => Json(serde_json::json!({
            "available": true,
            "tag": tag,
            "url": crate::common::consts::RELEASES_PAGE_URL,
        })),
        _ => Json(serde_json::json!({ "available": false })),
    }
}

async fn api_do_update() -> impl IntoResponse {
    // Verify the update and get the download URL synchronously before responding.
    let info =
        match tokio::task::spawn_blocking(version_check::check_for_update_with_url).await {
            Ok(Some(info)) => info,
            Ok(None) => {
                return Json(
                    serde_json::json!({ "ok": false, "error": "No update available or download URL not found" }),
                );
            }
            Err(e) => {
                return Json(serde_json::json!({ "ok": false, "error": e.to_string() }));
            }
        };

    // Spawn the download + install asynchronously so the response reaches the browser
    // before this process exits.
    tokio::task::spawn_blocking(move || {
        if let Err(e) = perform_self_update(info) {
            log::error!("Self-update failed: {e}");
        }
    });

    Json(serde_json::json!({ "ok": true }))
}

fn perform_self_update(info: version_check::UpdateInfo) -> anyhow::Result<()> {
    use anyhow::anyhow;

    log::info!("Downloading update {}...", info.tag);

    let response = minreq::get(&info.download_url)
        .with_header("User-Agent", "BarvazDNS")
        .with_timeout(120)
        .send()
        .map_err(|e| anyhow!("Download failed: {e}"))?;

    if response.status_code != 200 {
        return Err(anyhow!("Download failed: HTTP {}", response.status_code));
    }

    let new_binary = response.as_bytes().to_vec();

    let temp_path = std::env::temp_dir().join("BarvazDNS_update.exe");
    std::fs::write(&temp_path, &new_binary)
        .map_err(|e| anyhow!("Failed to write temp file: {e}"))?;

    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow!("Failed to determine current exe path: {e}"))?;
    let old_exe = current_exe.with_extension("exe.old");

    log::info!("Stopping service for update...");
    crate::service_manager::stop_service()
        .map_err(|e| anyhow!("Failed to stop service: {e}"))?;

    // Rename the running exe out of the way. On Windows, renaming a running
    // executable is allowed; open handles keep the inode alive.
    std::fs::rename(&current_exe, &old_exe)
        .map_err(|e| anyhow!("Failed to rename current exe: {e}"))?;

    // Copy (not rename) the new binary into place so cross-drive paths work.
    if let Err(e) = std::fs::copy(&temp_path, &current_exe) {
        let _ = std::fs::rename(&old_exe, &current_exe);
        return Err(anyhow!("Failed to place new exe: {e}"));
    }
    let _ = std::fs::remove_file(&temp_path);

    log::info!("Starting updated service...");
    if let Err(e) = crate::service_manager::start_service(true, true) {
        // Restore the original binary and try to bring the service back up.
        let _ = std::fs::remove_file(&current_exe);
        let _ = std::fs::rename(&old_exe, &current_exe);
        let _ = crate::service_manager::start_service(true, true);
        return Err(anyhow!("Failed to start updated service: {e}"));
    }

    let _ = std::fs::remove_file(&old_exe);

    log::info!("Update to {} complete — exiting old tray process", info.tag);
    // Give tokio a moment to flush the response to the browser before exiting.
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(2));
        std::process::exit(0);
    });

    Ok(())
}

const MAX_LOG_BYTES: u64 = 64 * 1024; // 64KB tail

async fn api_logs() -> impl IntoResponse {
    let path = match Config::get_config_directory_path() {
        Ok(mut p) => {
            p.push(format!("{LOG_FILE_BASENAME}_rCURRENT.log"));
            p
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": e.to_string() })),
            );
        }
    };

    match tokio::fs::metadata(&path).await {
        Ok(meta) => {
            let file_len = meta.len();
            let offset = file_len.saturating_sub(MAX_LOG_BYTES);
            match tokio::fs::read(&path).await {
                Ok(bytes) => {
                    let slice = &bytes[offset as usize..];
                    let text = String::from_utf8_lossy(slice);
                    // If we skipped bytes, drop the first partial line
                    let text = if offset > 0 {
                        text.split_once('\n').map_or("", |(_first, rest)| rest)
                    } else {
                        &text
                    };
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({ "lines": text.lines().collect::<Vec<_>>() })),
                    )
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": e.to_string() })),
                ),
            }
        }
        Err(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "lines": Vec::<String>::new() })),
        ),
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
