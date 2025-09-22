#!/usr/bin/env bash
set -euo pipefail

# Run a build inside the reusable Docker image
# Usage: ./scripts/run-windows-build.sh [--release]

IMAGE_NAME="goose/windows-build:latest"
HOST_PWD="$(pwd)"
CACHE_VOLUME="goose-windows-cache"
TARGET_DIR="target/x86_64-pc-windows-gnu/release"

PROFILE="--release"
if [ "${1-}" = "--debug" ]; then
  PROFILE=""
fi

# Create cache volume if it doesn't exist
if ! docker volume ls -q | grep -q "${CACHE_VOLUME}"; then
  echo "Creating docker volume ${CACHE_VOLUME} for cargo registry cache..."
  docker volume create "${CACHE_VOLUME}"
fi

# Run the build
docker run --rm \
  -v "${HOST_PWD}:/usr/src/myapp" \
  -v "${CACHE_VOLUME}:/usr/local/cargo/registry" \
  -w /usr/src/myapp \
  "${IMAGE_NAME}" \
  sh -c "rustup target add x86_64-pc-windows-gnu && \
    export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc && \
    export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++ && \
    export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar && \
    export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc && \
    export PKG_CONFIG_ALLOW_CROSS=1 && \
    export PROTOC=/usr/bin/protoc && \
    export PATH=/usr/bin:\$PATH && \
    protoc --version && \
    cargo build ${PROFILE} --target x86_64-pc-windows-gnu && \
    GCC_DIR=\$(ls -d /usr/lib/gcc/x86_64-w64-mingw32/*/ | head -n 1) && \
    cp \$GCC_DIR/libstdc++-6.dll /usr/src/myapp/${TARGET_DIR}/ && \
    cp \$GCC_DIR/libgcc_s_seh-1.dll /usr/src/myapp/${TARGET_DIR}/ && \
    cp /usr/x86_64-w64-mingw32/lib/libwinpthread-1.dll /usr/src/myapp/${TARGET_DIR}/"

echo "Windows build complete. Artifacts are in ${TARGET_DIR}" 
