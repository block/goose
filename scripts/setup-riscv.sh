#!/usr/bin/env bash
# Experimental, community-contributed RISC-V setup. This configuration is not
# officially supported by the goose project.

set -euo pipefail

echo "Checking RISC-V toolchain..."
if [ "$(uname -m)" = "riscv64" ]; then
    echo "   ✓ native RISC-V build - no cross toolchain needed"
elif command -v riscv64-linux-gnu-gcc > /dev/null 2>&1; then
    echo "   ✓ riscv64-linux-gnu-gcc found"
    echo "     For cross-compiling, export before building:"
    echo "     export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc"
else
    echo "   ⚠ riscv64-linux-gnu-gcc not found"
    echo "     Install it (e.g. 'sudo apt install gcc-riscv64-linux-gnu') and set:"
    echo "     export CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER=riscv64-linux-gnu-gcc"
fi

echo ""
echo "=== Toolchain check complete ==="
echo ""
echo "To build for RISC-V:"
echo "  cargo build --release --target riscv64gc-unknown-linux-gnu -p goose-cli --bin goose"
echo ""
echo "Output binary:"
echo "  target/riscv64gc-unknown-linux-gnu/release/goose"
