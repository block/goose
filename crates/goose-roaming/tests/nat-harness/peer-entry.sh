#!/bin/bash
# Peer entrypoint: route everything through the harness router instead of the
# (absent) docker gateway, then run the given command. Re-runs on container
# restart, so a crash-restarted host comes back with the same routing.
set -euo pipefail
ROUTER_IP="${1:?usage: peer-entry.sh <router-ip> <cmd...>}"
shift
ip route replace default via "$ROUTER_IP"
exec "$@"
