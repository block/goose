# Real-NAT soak harness

The in-crate tests (`path_upgrade.rs`) pin the relay→direct upgrade and dial
burst on **localhost**, where every candidate address is directly reachable.
This harness runs the same seams across a **real NAT**: two peers on isolated
subnets behind one Linux router doing full-cone NAT, a relay on the WAN side,
and every packet forced through router rules — the same-NAT topology from the
field reports on #10906 that a localhost test structurally cannot exercise.

```
                         wan 10.99.0.0/24
        ┌──────────────┐        │        ┌──────────────┐
        │ relay        │────────┼────────│ router (NAT) │
        │ 10.99.0.10   │        │        │ 10.99.0.2    │
        └──────────────┘                 └───┬──────┬───┘
                              lan-a          │      │          lan-b
                        192.168.101.0/24 ────┘      └──── 192.168.102.0/24
                        ┌──────────────┐                  ┌──────────────┐
                        │ host peer    │                  │ client peer  │
                        │ .101.10      │                  │ .102.10      │
                        └──────────────┘                  └──────────────┘
```

- The peer subnets are docker `--internal` networks; the router container is
  their only path anywhere. Direct LAN-to-LAN traffic between the peer
  subnets is dropped unless it was DNATed by the router first (AP-isolation
  semantics) — without that, iroh holepunches via the peers' private
  addresses and the NAT is never exercised.
- The router maps each peer's QUIC socket to a deterministic WAN port
  (full-cone shape) and runs in two modes: `nohairpin` (reflexive traffic
  from inside is not looped back — most consumer routers) and `hairpin`
  (NAT reflection enabled).
- The relay is the real `iroh-relay` server (plain HTTP/WebSocket transport,
  no TLS ceremony), spawned by the same harness binary.
- Trust bootstrap models the card/paste exchange over a shared volume: the
  host writes its connection card, clients drop their endpoint id, the host
  accepts every id it sees.

## Run it

```bash
cd crates/goose-roaming/tests/nat-harness
./run.sh              # build (cached in docker volumes) + full battery
./run.sh --no-build   # reuse the previously built binary/image
```

Needs docker with `NET_ADMIN` containers (stock Linux docker or a mac VM
runtime). First build compiles the iroh tree once; after that a full battery
is ~6 minutes. Results land in `results/results.jsonl` (one JSON object per
scenario) plus per-scenario logs.

## Scenarios

| scenario | what it measures | pass condition |
|---|---|---|
| `soak-nohairpin` | 300 numbered frames over one connection through the NAT | every frame echoes verbatim, in order — a silent drop fails at its frame number |
| `burst` | 8 parallel dials to one share across the NAT | all dials connect and echo independently |
| `cold-1..8` | fresh process → relay online → dial → first frame | reports the cold-path latency split (`online_ms` / `connect_ms` / `first_frame_ms`) |
| `soak-hairpin` | same soak after switching the router to NAT reflection | integrity as above; additionally reports whether a direct path was ever selected |
| `crash` | host SIGKILLed mid-soak, restarted ~10 s later | client reconnects unaided; reports outage duration; data-plane liveness (not just an accepted dial) closes the outage window |
| port audit | `ss` on both peers | no TCP listeners; only the bound QUIC UDP socket |

Every soak/crash result carries the connection's **path timeline**
(`PathEvent`s with timestamps) and a `direct_path_selected` flag, so upgrade
behavior across the NAT is a recorded measurement, not an assumption.

## Measured battery (2026-08-10, branch head f9c2268, apple-silicon docker VM)

| scenario | result |
|---|---|
| soak, `nohairpin` NAT | 300/300 frames verbatim in order; only path: `Relay(...)`; zero path events |
| soak, `hairpin` NAT | 300/300, identical — no upgrade attempted (see below) |
| burst | 8/8 parallel dials, connect p50 3 ms |
| cold ×8 | relay online ~215 ms; accepted dial 43–51 ms; first frame <1 ms |
| host SIGKILL + restart | no transport error surfaces — the in-flight frame hangs until the app echo timeout (10 s here), then reconnect + first frame in ~1.1 s |
| relay SIGKILL + restart | full outage (relay-only path, as expected); recovered ~3.1 s after the relay came back |
| port audit | no TCP listeners on either peer; only the bound QUIC UDP socket |

Latencies are virtual-network numbers (all containers share one VM) — the
harness measures topology behavior and integrity, not internet RTTs. The
crash rows record a real client-UX property: a SIGKILLed remote produces no
QUIC error, so failure detection falls to the application's own timeout.

## What to expect on current code (and why that's the point)

`RelaySettings::Custom` builds client relay configs with `quic: None`
(`relay.rs` → `RelayConfig::new(url, None)`), which per iroh's docs disables
QUIC address discovery against that relay. Without address discovery the
peers never learn their reflexive (post-NAT) addresses, so behind a real NAT
there are no usable direct candidates — local candidates are unroutable
(blocked LAN-to-LAN here; different sites in the field) and reflexive ones
are unknown. Since every production relay path (including the default
managed relays) goes through `RelaySettings::Custom`, the expected
measurement today is: **connections stay on the relay path in both router
modes, and the soak asserts that the relay path holds sustained traffic
without loss** — which is the availability property that actually matters
until address discovery is wired (e.g. a `RelayEntry` field for the relay's
QUIC address-discovery port). The localhost upgrade test stays green through
all of this because localhost candidates are directly reachable; that gap is
what this harness exists to close.

## Caveats

- netfilter full-cone + reflection is an approximation of consumer-router
  hairpin behavior — deliberately the *optimistic* case. A router that maps
  ports symmetrically is strictly harsher.
- The AP-isolation FORWARD drop models peers that cannot see each other's
  private addresses (guest wifi, corp subnets, different sites). Same-subnet
  peers would short-circuit via LAN candidates and are already covered by
  the localhost tests.
- This is a manual/nightly harness, not part of `cargo test`: it needs
  docker, `NET_ADMIN`, and minutes of wall clock. CI wiring (a nightly job)
  is straightforward if wanted.
