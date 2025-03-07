use std::path::Path;
use std::process::Command;

fn main() {
    // Skip build script for debug builds
    if std::env::var("PROFILE").unwrap_or("debug".to_string()) != "release" {
        return;
    }

    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let res_path = Path::new(&out_dir).join("icon.res");
        let icon_path = std::env::current_dir().unwrap().join("resources").join("icon.rc");

        let output = Command::new("rc.exe")
            .arg("/fo") // Output file option for rc.exe
            .arg(res_path.as_os_str())
            .arg(icon_path.as_os_str())
            .output()
            .expect("Failed to execute rc.exe");

        if !output.status.success() {
            panic!("rc.exe failed: {:?}", output);
        }

        println!("cargo:rustc-link-arg-bin=BarvazDNS={}", res_path.display());
        println!("cargo:rerun-if-changed=icon.rc");
    } else {
        panic!("OUT_DIR environment variable not set");
    }
}