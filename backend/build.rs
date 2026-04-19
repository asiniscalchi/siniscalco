use std::process::Command;

fn main() {
    let version = std::env::var("GIT_VERSION")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            let tag = Command::new("git")
                .args(["describe", "--tags", "--exact-match"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            tag.unwrap_or_else(|| {
                Command::new("git")
                    .args(["rev-parse", "--short", "HEAD"])
                    .output()
                    .ok()
                    .and_then(|o| String::from_utf8(o.stdout).ok())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
            })
        });

    println!("cargo:rustc-env=GIT_VERSION={version}");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/tags");
}
