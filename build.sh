#!/bin/bash
set -e

cd "$(dirname "$0")"

REMOTE_HOST="oomalogin@thanos-test-1-frame.frame.ooma.com"
REMOTE_DIR="~/goose"

TARGET=${1:-all}

if [[ "$TARGET" == "all" || "$TARGET" == "native" ]]; then
  echo "Building goose project (native)..."
  source bin/activate-hermit
  cargo build
fi

if [[ "$TARGET" == "all" || "$TARGET" == "linux" ]]; then
  echo "Syncing source to ${REMOTE_HOST}:${REMOTE_DIR}..."
  rsync -az --delete \
    --exclude='.git/' \
    --exclude='target/' \
    --exclude='.hermit/' \
    --exclude='ui/desktop/node_modules/' \
    --exclude='ui/desktop/out/' \
    --exclude='.goose/' \
    --exclude='tmp/' \
    --exclude='logs/' \
    --exclude='benchmark-*' \
    --exclude='do_not_version/' \
    . "${REMOTE_HOST}:${REMOTE_DIR}"

  echo "Building goose project (linux/amd64) on ${REMOTE_HOST}..."
  ssh "${REMOTE_HOST}" "
    set -e
    cd ${REMOTE_DIR}
    if ! command -v rustup &>/dev/null; then
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    fi
    source \"\$HOME/.cargo/env\"
    rustup target add x86_64-unknown-linux-gnu
    CARGO_INCREMENTAL=0 cargo build --release --target x86_64-unknown-linux-gnu -j4
  "

  echo "Fetching binary..."
  mkdir -p ./target
  rsync -az "${REMOTE_HOST}:${REMOTE_DIR}/target/x86_64-unknown-linux-gnu/release/goose" \
    ./target/goose-linux-amd64
  # Binary is fetched to ./target/goose-linux-amd64 on the local machine.
  # On the remote host it remains at ~/goose/target/x86_64-unknown-linux-gnu/release/goose
  echo "Binary saved to ./target/goose-linux-amd64"
fi

echo ""
echo "Build completed successfully!"
