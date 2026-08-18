#!/bin/bash
# NAT rules for the harness router. Both peers sit on isolated subnets behind
# this one box, which is the only path between them and to the WAN — the
# same-NAT topology where the field failures were reported.
#
#   router.sh nohairpin   full-cone NAT, but reflexive traffic from inside is
#                         not looped back (most consumer routers)
#   router.sh hairpin     full-cone NAT with NAT reflection, so peers can
#                         reach each other via their WAN mappings
#
# Direct LAN-to-LAN traffic between the peer subnets is dropped unless it was
# DNATed here first (AP-isolation semantics): without this, iroh holepunches
# via the peers' private addresses and the NAT is never exercised at all.
set -euo pipefail

MODE="${1:?usage: router.sh nohairpin|hairpin}"

WAN_IP=10.99.0.2
LAN_A=192.168.101.0/24
LAN_B=192.168.102.0/24
PEER_A=192.168.101.10
PEER_B=192.168.102.10
QUIC_PORT=7777
# Deterministic WAN mappings for the peers' QUIC sockets (full-cone shape).
MAP_A=17777
MAP_B=27777

WAN_IF=$(ip -o -4 addr show | awk -v ip="$WAN_IP" '$4 ~ ip"/" {print $2}')
[ -n "$WAN_IF" ] || { echo "cannot find WAN interface for $WAN_IP" >&2; exit 1; }

# run.sh sets this via `docker run --sysctl`; only set it when running
# somewhere that hasn't (an unprivileged container may not be allowed to).
if [ "$(cat /proc/sys/net/ipv4/ip_forward)" != "1" ]; then
  sysctl -qw net.ipv4.ip_forward=1
fi

iptables -t nat -F
iptables -F FORWARD
iptables -P FORWARD ACCEPT

case "$MODE" in
  nohairpin)
    iptables -t nat -A PREROUTING -i "$WAN_IF" -p udp -d "$WAN_IP" --dport "$MAP_A" \
      -j DNAT --to-destination "$PEER_A:$QUIC_PORT"
    iptables -t nat -A PREROUTING -i "$WAN_IF" -p udp -d "$WAN_IP" --dport "$MAP_B" \
      -j DNAT --to-destination "$PEER_B:$QUIC_PORT"
    ;;
  hairpin)
    iptables -t nat -A PREROUTING -p udp -d "$WAN_IP" --dport "$MAP_A" \
      -j DNAT --to-destination "$PEER_A:$QUIC_PORT"
    iptables -t nat -A PREROUTING -p udp -d "$WAN_IP" --dport "$MAP_B" \
      -j DNAT --to-destination "$PEER_B:$QUIC_PORT"
    # Reflected flows must also be source-NATed to the sender's WAN mapping,
    # or the receiver would reply directly to a private address it cannot
    # reach and the hairpin would half-work.
    iptables -t nat -A POSTROUTING -p udp -s "$LAN_A" -d "$PEER_B" --dport "$QUIC_PORT" \
      -j SNAT --to-source "$WAN_IP:$MAP_A"
    iptables -t nat -A POSTROUTING -p udp -s "$LAN_B" -d "$PEER_A" --dport "$QUIC_PORT" \
      -j SNAT --to-source "$WAN_IP:$MAP_B"
    ;;
  *)
    echo "unknown mode $MODE" >&2; exit 1;;
esac

iptables -t nat -A POSTROUTING -o "$WAN_IF" -p udp -s "$PEER_A" --sport "$QUIC_PORT" \
  -j SNAT --to-source "$WAN_IP:$MAP_A"
iptables -t nat -A POSTROUTING -o "$WAN_IF" -p udp -s "$PEER_B" --sport "$QUIC_PORT" \
  -j SNAT --to-source "$WAN_IP:$MAP_B"
iptables -t nat -A POSTROUTING -o "$WAN_IF" -j MASQUERADE

iptables -A FORWARD -s "$LAN_A" -d "$LAN_B" -m conntrack ! --ctstate DNAT -j DROP
iptables -A FORWARD -s "$LAN_B" -d "$LAN_A" -m conntrack ! --ctstate DNAT -j DROP

echo "ROUTER_READY mode=$MODE wan_if=$WAN_IF"
