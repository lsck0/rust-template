use std::{env, path::Path, process::Command};

const UNKNOWN: &str = "unknown";

fn git_output(arguments: &[&str]) -> Option<String> {
    let output = Command::new("git").args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    return Some(text.trim().to_owned());
}

fn git_rerun_if_changed() {
    let Some(git_dir) = git_output(&["rev-parse", "--absolute-git-dir"]) else {
        // a missing path reruns the script on every build, so the metadata appears once the repository is created
        println!("cargo::rerun-if-changed=.git");
        return;
    };
    let git_dir = Path::new(&git_dir);
    println!("cargo::rerun-if-changed={}", git_dir.join("HEAD").display());
    if let Some(head_ref) = git_output(&["symbolic-ref", "--quiet", "HEAD"]) {
        println!("cargo::rerun-if-changed={}", git_dir.join(head_ref).display());
    }
    if git_dir.join("packed-refs").exists() {
        println!("cargo::rerun-if-changed={}", git_dir.join("packed-refs").display());
    }
}

fn main() {
    git_rerun_if_changed();

    let version = git_output(&["describe", "--tags", "--always"]).unwrap_or_else(|| UNKNOWN.to_owned());
    let commit = git_output(&["rev-parse", "HEAD"]).unwrap_or_else(|| UNKNOWN.to_owned());
    let commit_time = git_output(&["show", "--no-patch", "--format=%cI", "HEAD"]).unwrap_or_else(|| UNKNOWN.to_owned());
    let profile = env::var("PROFILE").unwrap_or_else(|_| UNKNOWN.to_owned());
    let target = env::var("TARGET").unwrap_or_else(|_| UNKNOWN.to_owned());

    println!("cargo::rustc-env=BUILD_VERSION={version}");
    println!("cargo::rustc-env=BUILD_COMMIT={commit}");
    println!("cargo::rustc-env=BUILD_COMMIT_TIME={commit_time}");
    println!("cargo::rustc-env=BUILD_PROFILE={profile}");
    println!("cargo::rustc-env=BUILD_TARGET={target}");
}
