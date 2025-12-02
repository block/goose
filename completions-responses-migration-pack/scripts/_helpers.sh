#!/usr/bin/env bash
# Optional helpers library for local development of the starter pack.
# The published one-liner script is standalone and does not depend on this.

set -Eeuo pipefail

log() { printf "[*] %s\n" "$*"; }
info() { printf "[i] %s\n" "$*"; }
warn() { printf "[!] %s\n" "$*" >&2; }
err() { printf "[-] %s\n" "$*" >&2; }


