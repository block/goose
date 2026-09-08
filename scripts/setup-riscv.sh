#!/usr/bin/env bash
# Setup script for building goose on RISC-V
# This script vendors dependencies and applies necessary patches for V8 152.2.0

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VENDOR_DIR="$REPO_ROOT/vendor"

echo "=== Setting up goose for RISC-V build ==="
echo "Repository: $REPO_ROOT"
echo ""

# 1. Clone rusty_v8
echo "1. Cloning rusty_v8 v152.2.0..."
if [ -d "$VENDOR_DIR/rusty_v8" ]; then
    echo "   Already exists, skipping..."
else
    cd "$VENDOR_DIR"
    git clone --depth 1 --branch v152.2.0 https://github.com/denoland/rusty_v8.git
    rm -rf rusty_v8/.git
    echo "   ✓ Done"
fi

# 2. Vendor deno_core
echo "2. Vendoring deno_core 0.381.1..."
if [ -d "$VENDOR_DIR/deno_core" ]; then
    echo "   Already exists, skipping..."
else
    cd "$VENDOR_DIR"
    wget -q https://static.crates.io/crates/deno_core/deno_core-0.381.1.crate
    tar xzf deno_core-0.381.1.crate
    mv deno_core-0.381.1 deno_core
    rm deno_core-0.381.1.crate
    echo "   ✓ Done"
fi

# 3. Vendor serde_v8
echo "3. Vendoring serde_v8 0.290.0..."
if [ -d "$VENDOR_DIR/serde_v8" ]; then
    echo "   Already exists, skipping..."
else
    cd "$VENDOR_DIR"
    wget -q https://static.crates.io/crates/serde_v8/serde_v8-0.290.0.crate
    tar xzf serde_v8-0.290.0.crate
    mv serde_v8-0.290.0 serde_v8
    rm serde_v8-0.290.0.crate
    echo "   ✓ Done"
fi

# 4. vendor/v8
# Nothing to patch: the cloned vendor/rusty_v8 IS the `v8` crate at 152.2.0,
# so step 8 points `[patch.crates-io] v8` straight at it. The committed
# vendor/v8 shim is left alone (it stays a workspace member for cargo-machete
# but nothing depends on it once the patch is redirected).
echo "4. vendor/v8: using cloned rusty_v8 directly, nothing to patch"

# 5. Update deno_core v8 dependency
echo "5. Updating deno_core v8 dependency to 152.2.0..."
sed -i 's/version = "145\.0\.0"/version = "152.2.0"/' "$VENDOR_DIR/deno_core/Cargo.toml"
echo "   ✓ Done"

# 6. Update serde_v8 v8 dependency
echo "6. Updating serde_v8 v8 dependency to 152.2.0..."
sed -i 's/version = "145\.0\.0"/version = "152.2.0"/' "$VENDOR_DIR/serde_v8/Cargo.toml"
echo "   ✓ Done"

# 7. Apply deno_core source patches
echo "7. Applying deno_core source patches for V8 152 API..."

# Wrap .open() calls in unsafe
sed -i 's/let format_exception_cb = format_exception_cb\.open(scope);/let format_exception_cb = unsafe { format_exception_cb.open(scope) };/' "$VENDOR_DIR/deno_core/error.rs"
sed -i 's/let cb = cb\.open(tc_scope);/let cb = unsafe { cb.open(tc_scope) };/' "$VENDOR_DIR/deno_core/error.rs"
sed -i 's/let resolver = resolver_handle\.open(scope);/let resolver = unsafe { resolver_handle.open(scope) };/g' "$VENDOR_DIR/deno_core/modules/map.rs"
sed -i 's/let module = module_handle\.open(scope);/let module = unsafe { module_handle.open(scope) };/g' "$VENDOR_DIR/deno_core/modules/map.rs"
sed -i 's/let promise = pending_dyn_evaluate\.promise\.open(scope);/let promise = unsafe { pending_dyn_evaluate.promise.open(scope) };/g' "$VENDOR_DIR/deno_core/modules/map.rs"
sed -i 's/let _module = pending_dyn_evaluate\.module\.open(scope);/let _module = unsafe { pending_dyn_evaluate.module.open(scope) };/g' "$VENDOR_DIR/deno_core/modules/map.rs"
sed -i 's/let resolver = state\.resolver\.open(scope);/let resolver = unsafe { state.resolver.open(scope) };/g' "$VENDOR_DIR/deno_core/modules/map.rs"
sed -i 's/let ctx = self\.context()\.open(scope);/let ctx = unsafe { self.context().open(scope) };/g' "$VENDOR_DIR/deno_core/runtime/jsrealm.rs"
sed -i 's/let cb = function\.open(scope);/let cb = unsafe { function.open(scope) };/g' "$VENDOR_DIR/deno_core/runtime/jsruntime.rs"
sed -i 's/run_immediate_callbacks_cb\.as_ref()\.unwrap()\.open(tc_scope);/unsafe { run_immediate_callbacks_cb.as_ref().unwrap().open(tc_scope) };/g' "$VENDOR_DIR/deno_core/runtime/jsruntime.rs"
sed -i 's/let function = handler\.open(scope);/let function = unsafe { handler.open(scope) };/g' "$VENDOR_DIR/deno_core/runtime/jsruntime.rs"
sed -i 's/js_event_loop_tick_cb\.as_ref()\.unwrap()\.open(tc_scope);/unsafe { js_event_loop_tick_cb.as_ref().unwrap().open(tc_scope) };/g' "$VENDOR_DIR/deno_core/runtime/jsruntime.rs"

# Fix ops_builtin_v8.rs - multiline sed
sed -i '/cb_handle$/{ N; s/cb_handle\n      \.open(scope)/unsafe { cb_handle.open(scope) }/; }' "$VENDOR_DIR/deno_core/ops_builtin_v8.rs"

# Disable fast-call path in bindings.rs: replace the whole if/else block
# with the fallback assignment (c on an address range replaces all of it)
sed -i '/let template = if let Some(fast_function)/,/};/c\
  // Disable fast path for v8 152 - causes SIGILL on RISC-V snapshot creation\
  let template = builder.build(scope);' "$VENDOR_DIR/deno_core/runtime/bindings.rs"

# With the fast path disabled, the fast_fn binding above is unused; silence
# the resulting warning so clippy -D warnings stays clean.
sed -i 's/^  let (slow_fn, fast_fn) = /  let (slow_fn, _fast_fn) = /' "$VENDOR_DIR/deno_core/runtime/bindings.rs"

# Update WasmStreaming generic
sed -i 's/pub struct WasmStreamingResource(pub(crate) RefCell<v8::WasmStreaming>);/pub struct WasmStreamingResource(pub(crate) RefCell<v8::WasmStreaming<false>>);/' "$VENDOR_DIR/deno_core/ops_builtin.rs"

# ICU 77 -> 78: V8 152 bundles ICU 78, so the initializer name AND the data
# blob must both move. deno_core_icudata 0.78.0 ships the matching ICU 78 data;
# passing the 0.77.0 blob to set_common_data_78 fails V8 initialization.
sed -i 's/set_common_data_77/set_common_data_78/' "$VENDOR_DIR/deno_core/runtime/setup.rs"
sed -i 's/^version = "0\.77\.0"$/version = "0.78.0"/' "$VENDOR_DIR/deno_core/Cargo.toml"

# Remove --no-validate-asm flag
sed -i 's/" --no-validate-asm",//' "$VENDOR_DIR/deno_core/runtime/setup.rs"

echo "   ✓ Done"

# 8. Update root Cargo.toml
echo "8. Updating root Cargo.toml..."
cd "$REPO_ROOT"

# Add rusty_v8 to workspace exclusions (after the members array).
# Use awk for portability: sed multi-line behaviour differs between
# GNU and BSD implementations.
if ! grep -q 'exclude = \["vendor/rusty_v8"\]' Cargo.toml; then
    awk '
        /^members = \[/ { in_members = 1 }
        in_members && /^\]$/ {
            print
            print "exclude = [\"vendor/rusty_v8\"]"
            in_members = 0
            next
        }
        { print }
    ' Cargo.toml > Cargo.toml.new
    mv Cargo.toml.new Cargo.toml
fi

# Relax ICU pins
sed -i 's/icu_calendar = { version = "=2\.1\.1"/icu_calendar = { version = ">=2.1"/' Cargo.toml
sed -i 's/icu_locale = { version = "=2\.1\.1"/icu_locale = { version = ">=2.1"/' Cargo.toml

# Add deno_core/serde_v8 patches and repoint the existing `v8` patch at the
# cloned rusty_v8 (same crate name, version 152.2.0). Multi-line sed `a\` is
# fragile across sed implementations, so use awk.
if ! grep -q 'vendor/deno_core' Cargo.toml; then
    awk '
        /^\[patch\.crates-io\]$/ {
            print
            print "deno_core = { path = \"vendor/deno_core\" }"
            print "serde_v8 = { path = \"vendor/serde_v8\" }"
            next
        }
        /^v8 = \{ path = "vendor\/v8" \}$/ {
            print "v8 = { path = \"vendor/rusty_v8\" }"
            next
        }
        { print }
    ' Cargo.toml > Cargo.toml.new
    mv Cargo.toml.new Cargo.toml
fi

echo "   ✓ Done"

# 9. Update crates/goose/Cargo.toml
echo "9. Updating crates/goose/Cargo.toml..."
sed -i 's/icu_calendar = { version = "=2\.1\.1"/icu_calendar = { version = ">=2.1"/' "$REPO_ROOT/crates/goose/Cargo.toml"
sed -i 's/icu_locale = { version = "=2\.1\.1"/icu_locale = { version = ">=2.1"/' "$REPO_ROOT/crates/goose/Cargo.toml"
echo "   ✓ Done"

# Note: update.rs already handles RISC-V (asset name + self-update bail) in
# the repo, so no patching is needed here.

# 10. Update dependencies
echo "10. Updating Cargo dependencies..."
cd "$REPO_ROOT"
cargo update --quiet
echo "   ✓ Done"

# 11. Verify toolchain is available.
echo "11. Checking RISC-V toolchain..."
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
echo "=== Setup complete! ==="
echo ""
echo "To build for RISC-V:"
echo "  cargo build --release --target riscv64gc-unknown-linux-gnu -p goose-cli --bin goose"
echo ""
echo "Output binary:"
echo "  target/riscv64gc-unknown-linux-gnu/release/goose"
