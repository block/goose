# Runtime image for all four harness roles (relay / router / host / client).
# The binary is built separately (see run.sh) so the docker context stays tiny
# and cargo caching works across runs.
FROM debian:bookworm-slim
RUN apt-get update -qq \
    && apt-get install -y -qq --no-install-recommends \
        iproute2 iptables conntrack ca-certificates procps \
    && rm -rf /var/lib/apt/lists/*
COPY bin/nat_harness /usr/local/bin/nat_harness
COPY router.sh peer-entry.sh /usr/local/bin/
RUN chmod +x /usr/local/bin/nat_harness /usr/local/bin/router.sh /usr/local/bin/peer-entry.sh
