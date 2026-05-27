#!/usr/bin/env sh
set -eu

package_manifest="crates/rusty-castle/Cargo.toml"

usage() {
    cat >&2 <<EOF
usage: $0 [patch|minor|major|X.Y.Z]

Creates a release commit and annotated vX.Y.Z tag, then pushes both to origin.
Run from a clean worktree on the branch you want to release.
EOF
}

current_version() {
    cargo pkgid -p rusty-castle | sed 's/.*#//'
}

bump_version() {
    version="$1"
    bump="$2"

    major="${version%%.*}"
    rest="${version#*.}"
    minor="${rest%%.*}"
    patch="${rest#*.}"

    case "$bump" in
        major)
            major=$((major + 1))
            minor=0
            patch=0
            ;;
        minor)
            minor=$((minor + 1))
            patch=0
            ;;
        patch)
            patch=$((patch + 1))
            ;;
        *)
            printf '%s\n' "$bump"
            return
            ;;
    esac

    printf '%s.%s.%s\n' "$major" "$minor" "$patch"
}

require_semver() {
    case "$1" in
        *[!0-9.]* | *.*.*.* | .* | *. | *..*)
            return 1
            ;;
    esac

    major="${1%%.*}"
    rest="${1#*.}"
    minor="${rest%%.*}"
    patch="${rest#*.}"

    [ "$major" != "$1" ] \
        && [ "$minor" != "$rest" ] \
        && [ -n "$major" ] \
        && [ -n "$minor" ] \
        && [ -n "$patch" ]
}

confirm() {
    prompt="$1"
    printf '%s [y/N] ' "$prompt"
    read -r answer
    case "$answer" in
        y | Y | yes | YES)
            return 0
            ;;
        *)
            return 1
            ;;
    esac
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
    usage
    exit 0
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
    printf 'release aborted: worktree has uncommitted changes.\n' >&2
    exit 1
fi

current="$(current_version)"
choice="${1:-}"

if [ -z "$choice" ]; then
    printf 'Current rusty-castle version: %s\n' "$current"
    printf 'Select release type [patch/minor/major/custom]: '
    read -r choice
    if [ "$choice" = "custom" ]; then
        printf 'Enter version X.Y.Z: '
        read -r choice
    fi
fi

case "$choice" in
    patch | minor | major)
        next="$(bump_version "$current" "$choice")"
        ;;
    *)
        next="$choice"
        ;;
esac

if ! require_semver "$next"; then
    printf 'release aborted: expected a SemVer version like 0.1.2, got "%s".\n' "$next" >&2
    exit 1
fi

tag="v${next}"

if git rev-parse -q --verify "refs/tags/${tag}" >/dev/null; then
    printf 'release aborted: tag %s already exists locally.\n' "$tag" >&2
    exit 1
fi

cat <<EOF

Release plan
  current version: ${current}
  next version:    ${next}
  git tag:         ${tag}

This will update ${package_manifest} and Cargo.lock if needed, run tests,
create an annotated tag, and push the release to origin. If the requested
version is already committed, no release commit will be created.
EOF

confirm "Continue?" || exit 0

tmp="${package_manifest}.tmp"
sed "0,/^version = \".*\"/s//version = \"${next}\"/" "$package_manifest" > "$tmp"
mv "$tmp" "$package_manifest"

cargo check --workspace --all-features
cargo test --workspace --all-features

git diff -- "$package_manifest" Cargo.lock
confirm "Create release commit if needed and push ${tag}?" || exit 0

git add "$package_manifest" Cargo.lock

if git diff --cached --quiet; then
    printf 'No version file changes to commit; tagging current HEAD.\n'
else
    git commit -m "chore: release ${tag}"
fi

git tag -a "$tag" -m "$tag"

git push origin HEAD
git push origin "$tag"

cat <<EOF

Release ${tag} pushed.
GitHub Actions will create the GitHub Release and publish Docker images.
EOF
