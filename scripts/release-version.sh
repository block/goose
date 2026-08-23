#!/usr/bin/env bash
# Resolve the desktop/CLI release version to a bare semver (no leading v).
#
# Order:
#   1. first argument, or RELEASE_VERSION
#   2. GitHub tag on GITHUB_REF (refs/tags/vX.Y.Z)
#   3. Cargo.toml workspace.package.version
#
# Flags:
#   --from-cargo   print Cargo.toml only (ignore env/tag/arg)
#   --check        fail if Cargo.toml != ui/desktop/package.json
#   --self-test    run mismatch/match checks against temp copies

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

normalize() {
  local v="${1:-}"
  v="${v#v}"
  v="${v#"${v%%[![:space:]]*}"}"
  v="${v%"${v##*[![:space:]]}"}"
  printf '%s' "$v"
}

from_cargo() {
  local toml="${1:-$ROOT_DIR/Cargo.toml}"
  local version
  version="$(grep -E '^version = "' "$toml" | head -n1 | sed -E 's/^version = "([^"]+)".*/\1/')"
  if [[ -z "$version" ]]; then
    echo "FAIL: could not read workspace.package.version from $toml" >&2
    return 1
  fi
  printf '%s' "$version"
}

from_package_json() {
  local pkg="${1:-$ROOT_DIR/ui/desktop/package.json}"
  node -p 'require(process.argv[1]).version' "$pkg"
}

check_consistency() {
  local cargo_toml="${1:-$ROOT_DIR/Cargo.toml}"
  local package_json="${2:-$ROOT_DIR/ui/desktop/package.json}"
  local cargo pkg
  cargo="$(from_cargo "$cargo_toml")"
  pkg="$(from_package_json "$package_json")"
  if [[ "$cargo" != "$pkg" ]]; then
    echo "FAIL: Cargo.toml version ($cargo) != ui/desktop/package.json ($pkg)" >&2
    return 1
  fi
  echo "OK  Cargo.toml and ui/desktop/package.json both $cargo"
}

self_test() {
  local tmp
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' RETURN
  printf '[workspace.package]\nversion = "9.9.9"\n' >"$tmp/Cargo.toml"
  printf '{"version":"9.9.9"}\n' >"$tmp/package.json"
  check_consistency "$tmp/Cargo.toml" "$tmp/package.json"

  printf '{"version":"0.0.1"}\n' >"$tmp/package.json"
  if check_consistency "$tmp/Cargo.toml" "$tmp/package.json"; then
    echo "FAIL: expected mismatch to fail" >&2
    return 1
  fi
  echo "OK  self-test: mismatch fails, match passes"
}

resolve() {
  local explicit
  explicit="$(normalize "${1:-${RELEASE_VERSION:-}}")"
  if [[ -n "$explicit" ]]; then
    printf '%s' "$explicit"
    return 0
  fi

  if [[ "${GITHUB_REF:-}" == refs/tags/v* ]]; then
    local tag="${GITHUB_REF_NAME:-${GITHUB_REF#refs/tags/}}"
    printf '%s' "$(normalize "$tag")"
    return 0
  fi

  from_cargo
}

case "${1:-}" in
  --from-cargo)
    from_cargo
    echo
    ;;
  --check)
    check_consistency
    ;;
  --self-test)
    self_test
    ;;
  *)
    resolve "${1:-}"
    echo
    ;;
esac
