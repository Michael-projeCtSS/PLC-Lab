//! Build metadata for the About page: the exact source commit, whether the checkout had
//! changes that commit does not contain, and when this executable was built. Read from
//! git at build time; "unknown" when git is not available.
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn git(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn main() {
    let commit = git(&["rev-parse", "HEAD"]).unwrap_or_default();
    let dirty = git(&["status", "--porcelain"]).map(|s| !s.is_empty());
    let built = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    println!("cargo:rustc-env=PLC_LAB_SOURCE_COMMIT={commit}");
    println!(
        "cargo:rustc-env=PLC_LAB_SOURCE_DIRTY={}",
        match dirty {
            Some(true) => "dirty",
            Some(false) => "clean",
            None => "unknown",
        }
    );
    println!("cargo:rustc-env=PLC_LAB_BUILT_AT_UNIX={built}");
    // Run again when the commit or the sources change, so the record stays true.
    if let Some(dir) = git(&["rev-parse", "--absolute-git-dir"]) {
        for file in ["HEAD", "index", "packed-refs"] {
            println!("cargo:rerun-if-changed={dir}/{file}");
        }
        if let Some(reference) = git(&["symbolic-ref", "-q", "HEAD"]) {
            println!("cargo:rerun-if-changed={dir}/{reference}");
        }
    }
    if let Some(top) = git(&["rev-parse", "--show-toplevel"]) {
        for source in [
            "crates",
            "languages",
            "packages",
            "knowledge",
            "apps/desktop/src",
            "apps/desktop/src-tauri/src",
        ] {
            println!("cargo:rerun-if-changed={top}/{source}");
        }
    }
    tauri_build::build();
}
