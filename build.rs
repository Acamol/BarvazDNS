fn main() {
    // Skip build script for debug builds
    if std::env::var("PROFILE").unwrap_or("debug".to_string()) != "release" {
        return;
    }

    embed_resource::compile("resources/icon.rc", embed_resource::NONE);
}