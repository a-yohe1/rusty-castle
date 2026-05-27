#!/usr/bin/env sh
set -eu

mode="${1:-cargo}"
package_version="$(cargo pkgid -p rusty-castle | sed 's/.*#//')"
revision="$(git rev-parse --short=12 HEAD 2>/dev/null || printf unknown)"
tag="$(git describe --tags --exact-match --match 'v[0-9]*' 2>/dev/null || true)"

case "$tag" in
  v[0-9]*.[0-9]*.[0-9]*)
    version="${tag#v}"
    ;;
  *)
    version="${package_version}-dev+g${revision}"
    ;;
esac

case "$mode" in
  cargo)
    printf '%s\n' "$version"
    ;;
  docker)
    printf '%s\n' "$version" | tr '+' '-'
    ;;
  revision)
    printf '%s\n' "$revision"
    ;;
  *)
    printf 'usage: %s [cargo|docker|revision]\n' "$0" >&2
    exit 2
    ;;
esac
