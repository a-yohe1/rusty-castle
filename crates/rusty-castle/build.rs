use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=RUSTY_CASTLE_VERSION");
    println!("cargo:rerun-if-env-changed=RUSTY_CASTLE_REVISION");
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs/heads");
    println!("cargo:rerun-if-changed=../../.git/refs/tags");

    let package_version = env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION is set by cargo");
    let revision = non_empty_env("RUSTY_CASTLE_REVISION")
        .or_else(git_short_revision)
        .unwrap_or_else(|| "unknown".into());
    let version = non_empty_env("RUSTY_CASTLE_VERSION")
        .or_else(|| exact_semver_tag().map(|tag| tag.trim_start_matches('v').to_owned()))
        .unwrap_or_else(|| format!("{package_version}-dev+g{revision}"));

    println!("cargo:rustc-env=RUSTY_CASTLE_VERSION={version}");
    println!("cargo:rustc-env=RUSTY_CASTLE_REVISION={revision}");
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn exact_semver_tag() -> Option<String> {
    let tag = git(["describe", "--tags", "--exact-match", "--match", "v[0-9]*"])?;
    is_semver_tag(&tag).then_some(tag)
}

fn git_short_revision() -> Option<String> {
    git(["rev-parse", "--short=12", "HEAD"])
}

fn git<const N: usize>(args: [&str; N]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn is_semver_tag(tag: &str) -> bool {
    let version = tag.strip_prefix('v').unwrap_or(tag);
    let mut parts = version.splitn(3, '.');
    let Some(major) = parts.next() else {
        return false;
    };
    let Some(minor) = parts.next() else {
        return false;
    };
    let Some(patch_and_suffix) = parts.next() else {
        return false;
    };
    let patch = patch_and_suffix
        .split_once(['-', '+'])
        .map_or(patch_and_suffix, |(patch, _)| patch);

    [major, minor, patch]
        .iter()
        .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}
