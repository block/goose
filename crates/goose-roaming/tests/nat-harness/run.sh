#!/bin/bash
# Real-NAT soak for goose-roaming. Topology: a relay on a "WAN" network, and
# two peers on isolated subnets behind one NAT router container — the same-NAT
# shape the field reports on #10906 came from, which the localhost CI tests
# cannot exercise. Runs the full scenario battery and leaves machine-readable
# results in results/.
#
#   ./run.sh            build (cached) + full battery
#   ./run.sh --no-build reuse the last built binary/image
#
# Needs: docker able to run containers with NET_ADMIN (any stock linux docker
# or a mac VM runtime). Total runtime after the first cached build: ~6 min.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/../../../.." && pwd)"
RESULTS="$HERE/results"
PREFIX=goose-nat
RELAY_URL=http://10.99.0.10:3340
RUST_IMAGE=rust:1.96-bookworm

log() { printf '\n== %s ==\n' "$*"; }

cleanup() {
  docker rm -f "$PREFIX-relay" "$PREFIX-router" "$PREFIX-host" "$PREFIX-client" >/dev/null 2>&1 || true
  docker network rm "$PREFIX-wan" "$PREFIX-lan-a" "$PREFIX-lan-b" >/dev/null 2>&1 || true
  docker volume rm "$PREFIX-shared" >/dev/null 2>&1 || true
}
trap cleanup EXIT

wait_log() { # container pattern timeout_secs
  local i
  for ((i = 0; i < $3 * 2; i++)); do
    if docker logs "$1" 2>&1 | grep -q "$2"; then return 0; fi
    sleep 0.5
  done
  echo "timed out waiting for '$2' in $1 logs:" >&2
  docker logs "$1" >&2 || true
  return 1
}

client() { # scenario extra-args...
  local scenario=$1
  shift
  docker exec "$PREFIX-client" nat_harness client --shared /shared \
    --relay-url "$RELAY_URL" --scenario "$scenario" "$@"
}

if [ "${1:-}" != "--no-build" ]; then
  log "building harness binary (cargo, cached in docker volumes)"
  docker volume create "$PREFIX-cargo" >/dev/null
  docker volume create "$PREFIX-target" >/dev/null
  docker image inspect "$PREFIX-builder" >/dev/null 2>&1 \
    || docker build -t "$PREFIX-builder" - <<EOF
FROM $RUST_IMAGE
RUN apt-get update -qq && apt-get install -y -qq cmake clang >/dev/null && rm -rf /var/lib/apt/lists/*
EOF
  docker run --rm -v "$REPO_ROOT:/src" -v "$PREFIX-cargo:/cargo-home" \
    -v "$PREFIX-target:/target" -e CARGO_HOME=/cargo-home -w /src \
    "$PREFIX-builder" cargo build --release -p goose-roaming \
    --example nat_harness --target-dir /target
  mkdir -p "$HERE/bin"
  # Unlink before extracting: copying over a live executable gets running
  # processes SIGKILLed on macOS (Code Signature Invalid) — see AGENTS.md.
  rm -f "$HERE/bin/nat_harness"
  docker run --rm -v "$PREFIX-target:/target" -v "$HERE/bin:/out" \
    debian:bookworm-slim cp /target/release/examples/nat_harness /out/
  log "building runtime image"
  docker build -t "$PREFIX-harness" "$HERE"
fi

mkdir -p "$RESULTS"
: >"$RESULTS/results.jsonl"

log "creating networks and shared volume"
cleanup
docker network create --subnet 10.99.0.0/24 "$PREFIX-wan" >/dev/null
docker network create --internal --subnet 192.168.101.0/24 "$PREFIX-lan-a" >/dev/null
docker network create --internal --subnet 192.168.102.0/24 "$PREFIX-lan-b" >/dev/null
docker volume create "$PREFIX-shared" >/dev/null

log "starting relay (WAN) and NAT router"
docker run -d --name "$PREFIX-relay" --network "$PREFIX-wan" --ip 10.99.0.10 \
  "$PREFIX-harness" nat_harness relay --bind 0.0.0.0:3340 >/dev/null
docker create --name "$PREFIX-router" --cap-add NET_ADMIN \
  --sysctl net.ipv4.ip_forward=1 --network "$PREFIX-wan" --ip 10.99.0.2 \
  "$PREFIX-harness" sleep infinity >/dev/null
docker network connect --ip 192.168.101.2 "$PREFIX-lan-a" "$PREFIX-router"
docker network connect --ip 192.168.102.2 "$PREFIX-lan-b" "$PREFIX-router"
docker start "$PREFIX-router" >/dev/null
docker exec "$PREFIX-router" router.sh nohairpin
wait_log "$PREFIX-relay" RELAY_READY 20

log "starting host (lan-a) and client (lan-b) behind the NAT"
docker run -d --name "$PREFIX-host" --cap-add NET_ADMIN \
  --network "$PREFIX-lan-a" --ip 192.168.101.10 -v "$PREFIX-shared:/shared" \
  "$PREFIX-harness" peer-entry.sh 192.168.101.2 \
  nat_harness host --shared /shared --relay-url "$RELAY_URL" >/dev/null
docker run -d --name "$PREFIX-client" --cap-add NET_ADMIN \
  --network "$PREFIX-lan-b" --ip 192.168.102.10 -v "$PREFIX-shared:/shared" \
  "$PREFIX-harness" peer-entry.sh 192.168.102.2 sleep infinity >/dev/null
wait_log "$PREFIX-host" HOST_READY 40

run_scenario() { # name cmd...
  local name=$1
  shift
  log "scenario: $name"
  "$@" | tee "$RESULTS/$name.log" | grep '^RESULT ' | sed "s/^RESULT //" \
    | while read -r line; do
        echo "$line" | sed "s/{/{\"name\":\"$name\",/" >>"$RESULTS/results.jsonl"
      done
}

run_scenario soak-nohairpin client soak --frames 300
run_scenario burst client burst --dials 8
for i in 1 2 3 4 5 6 7 8; do
  run_scenario "cold-$i" client cold
done

log "switching router to hairpin NAT (and flushing conntrack)"
docker exec "$PREFIX-router" router.sh hairpin
docker exec "$PREFIX-router" conntrack -F 2>/dev/null || true
sleep 10

run_scenario soak-hairpin client soak --frames 300

crash_scenario() { # name victim-container
  local name=$1 victim=$2
  log "scenario: $name (SIGKILL $victim 25s after the measured loop starts)"
  client crash --duration-secs 90 >"$RESULTS/$name.log" 2>&1 &
  local pid=$!
  # Anchor the kill timer on the client's data-plane-live marker — killing
  # during a slow setup would let the scenario pass without measuring anything.
  local i
  for ((i = 0; i < 240; i++)); do
    grep -q CRASH_RUNNING "$RESULTS/$name.log" 2>/dev/null && break
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  if ! grep -q CRASH_RUNNING "$RESULTS/$name.log" 2>/dev/null; then
    echo "$name: crash client never reached its measured loop" >&2
    cat "$RESULTS/$name.log" >&2 || true
    exit 1
  fi
  sleep 25
  docker kill -s KILL "$victim" >/dev/null
  sleep 10
  docker start "$victim" >/dev/null
  # The client exits non-zero if it never reconnects — that IS the recovery
  # regression this scenario exists to catch, so it must fail the run.
  if ! wait "$pid"; then
    echo "$name: client did not recover (see $RESULTS/$name.log)" >&2
    exit 1
  fi
  local result
  result=$(grep '^RESULT ' "$RESULTS/$name.log" || true)
  if [ -z "$result" ]; then
    echo "$name: client exited cleanly but recorded no RESULT" >&2
    exit 1
  fi
  echo "$result" | sed 's/^RESULT //' \
    | sed "s/{/{\"name\":\"$name\",/" >>"$RESULTS/results.jsonl"
  # A crash run whose kill window produced no outage measured nothing.
  if ! echo "$result" | grep -q '"outage_ms"'; then
    echo "$name: no outage recorded — the kill did not land inside the measured loop" >&2
    exit 1
  fi
}

crash_scenario crash "$PREFIX-host"
crash_scenario relay-crash "$PREFIX-relay"

log "NAT rule counters (evidence the QUIC flows traversed the router mappings)"
docker exec "$PREFIX-router" iptables -t nat -vnL | tee "$RESULTS/nat-counters.log"

log "port audit (expect: no TCP listeners, exactly the bound QUIC UDP socket)"
# The scenario execs have all exited by now, so without a live client the
# client-side audit would pass vacuously on an empty socket set. Hold a
# client data plane open (the crash scenario with nothing killed is just a
# resilient soak) and audit while it runs.
client crash --duration-secs 25 >"$RESULTS/audit-hold.log" 2>&1 &
AUDIT_PID=$!
for ((i = 0; i < 240; i++)); do
  grep -q CRASH_RUNNING "$RESULTS/audit-hold.log" 2>/dev/null && break
  kill -0 "$AUDIT_PID" 2>/dev/null || break
  sleep 0.5
done
if ! grep -q CRASH_RUNNING "$RESULTS/audit-hold.log" 2>/dev/null; then
  echo "port audit hold client never came up" >&2
  cat "$RESULTS/audit-hold.log" >&2 || true
  exit 1
fi
# Allowed sockets: docker's embedded DNS on 127.0.0.11, the deliberately
# bound QUIC v4 socket (required, both peers), and iroh's default v6 UDP
# transport. Anything else listening is an audit failure, not a log line.
for peer in host client; do
  echo "--- $PREFIX-$peer"
  docker exec "$PREFIX-$peer" sh -c 'echo "tcp listeners:"; ss -Hltn; echo "udp sockets:"; ss -Hlun' \
    | tee "$RESULTS/ports-$peer.log"
  bad_tcp=$(docker exec "$PREFIX-$peer" ss -Hltn | awk '$4 !~ /^127\.0\.0\.11:/' || true)
  bad_udp=$(docker exec "$PREFIX-$peer" ss -Hlun \
    | awk '$4 !~ /^127\.0\.0\.11:/ && $4 != "0.0.0.0:7777" && $4 !~ /^\[::\]:/' || true)
  # iroh's default v6 transport binds one wildcard socket on an ephemeral
  # port, so the v6 allowance is bounded by count rather than port.
  v6_udp=$(docker exec "$PREFIX-$peer" ss -Hlun | awk '$4 ~ /^\[::\]:/' | wc -l | tr -d ' ')
  if [ -n "$bad_tcp" ] || [ -n "$bad_udp" ] || [ "$v6_udp" -gt 1 ]; then
    echo "port audit FAILED on $PREFIX-$peer (wildcard v6 udp sockets: $v6_udp):" >&2
    printf '%s\n%s\n' "$bad_tcp" "$bad_udp" >&2
    exit 1
  fi
  if ! docker exec "$PREFIX-$peer" ss -Hlun | awk '$4 == "0.0.0.0:7777"' | grep -q .; then
    echo "port audit FAILED on $PREFIX-$peer: expected QUIC socket 0.0.0.0:7777 not bound" >&2
    exit 1
  fi
done
wait "$AUDIT_PID" || { echo "port audit hold client failed" >&2; exit 1; }
echo "port audit OK"

log "results ($RESULTS/results.jsonl)"
cat "$RESULTS/results.jsonl"
