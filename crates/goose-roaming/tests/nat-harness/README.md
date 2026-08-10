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
| `crash` | host SIGKILLed mid-soak, restarted ~10 s later | client reconnects unaided; reports outage duration; data-plane liveness closes the outage window. The kill anchors on the data-plane-live marker; a host crash must always produce an outage |
| `relay-crash` | relay SIGKILLed mid-soak | if the connection has upgraded to a direct path, killing the relay produces **no** outage (relay off the data plane) — recorded as the expected result; a relay crash with no direct path and no outage fails the run |
| port audit | `ss` on both peers, run while a client data plane is live | no TCP listeners; the bound QUIC UDP socket must be present on both peers and nothing else may listen |

Every soak/crash result carries the connection's **path timeline**
(`PathEvent`s with timestamps) and a `direct_path_selected` flag, so upgrade
behavior across the NAT is a recorded measurement, not an assumption.

## Measured battery (2026-08-10, branch head 3ea6c8b8 = QAD fix, apple-silicon docker VM)

The harness relay runs a QUIC address-discovery (QAD) endpoint on the default
port alongside the plain-HTTP relay, so peers behind the NAT can learn their
reflexive addresses — which is what the QAD fix (`3ea6c8b8`) needs to have
anything to query.

| scenario | result |
|---|---|
| soak, `nohairpin` NAT | 300/300 frames verbatim in order; stays on `Relay(...)` (no reflection + AP-isolation ⇒ no reachable direct candidate) |
| soak, `hairpin` NAT | 300/300; **upgrades to a direct path** — `paths_final` shows `Ip(10.99.0.2:17777) selected`, `direct_path_selected: true`. Upgrade lands within ~3 ms of connect once QAD supplies the reflexive address |
| burst | 8/8 parallel dials |
| cold ×8 | relay online ~215 ms; accepted dial ~45 ms; first frame <1 ms |
| host SIGKILL + restart | connection is on the direct path; killing the host still breaks it (host is an endpoint). No QUIC error surfaces, so `outage_ms` counts the app echo timeout + reconnect (~11 s) |
| relay SIGKILL + restart | **no outage** — the connection had upgraded to the direct path, so the relay is no longer on the data plane. Relay death is invisible to an already-direct connection |
| port audit | no TCP listeners on either peer; only the bound QUIC UDP socket |

Latencies are virtual-network numbers (all containers share one VM) — the
harness measures topology behavior and integrity, not internet RTTs.

## What this verifies (QAD fix `3ea6c8b8`)

Before the fix, `RelaySettings::Custom` built `RelayConfig::new(url, None)` —
`quic: None` disables QUIC address discovery, so a NAT'd peer never learned
its reflexive address and no direct candidate ever existed: the connection
stayed relay-only in **both** router modes, `path_events` empty. Every
production relay path (including the default managed relays) goes through
`Custom`, so direct upgrade only worked where local candidates were directly
reachable (same LAN / localhost) — which is exactly why the localhost
`path_upgrade.rs` test stayed green through the gap.

After the fix (`RelayEntry.qad_port` + `RelayConfig::new(url, Some(quic))`),
this harness confirms the real-NAT behavior end to end:

- **hairpin mode: the direct upgrade now lands.** QAD gives each peer its
  reflexive address; the reflection path lets the hole punch complete; the
  connection selects `Ip(...)` within milliseconds and carries traffic
  directly, relay off the data plane.
- **relay independence: killing the relay after the upgrade causes no
  outage** — the direct path is unaffected, so the relay is no longer a
  single point of failure once a connection has upgraded.
- **nohairpin mode still stays relay**, correctly: with AP-isolation blocking
  the LAN-direct path and a non-reflecting NAT blocking the reflexive path,
  there is genuinely no reachable direct candidate. This is the honest hard
  case, not a regression.

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
