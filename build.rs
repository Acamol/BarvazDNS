fn main() {
    // Point git to the committed hooks directory so pre-push checks run automatically.
    // This is a no-op if already configured, and harmless outside a git repo.
    let _ = std::process::Command::new("git")
        .args(["config", "core.hooksPath", ".githooks"])
        .status();

    // Only embed resources (icon + admin manifest) in release builds.
    // In debug, this avoids requiring elevation to run tests.
    if std::env::var("PROFILE").unwrap_or("debug".to_string()) != "release" {
        return;
    }

    embed_resource::compile("resources/app.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
