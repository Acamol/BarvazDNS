fn main() {
    // Point git to the committed hooks directory so pre-push checks run automatically.
    // This is a no-op if already configured, and harmless outside a git repo.
    let _ = std::process::Command::new("git")
        .args(["config", "core.hooksPath", ".githooks"])
        .status();

    // Skip resource embedding for debug builds
    if std::env::var("PROFILE").unwrap_or("debug".to_string()) != "release" {
        return;
    }

    embed_resource::compile("resources/icon.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
