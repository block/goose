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

# 4. Patch vendor/v8
echo "4. Patching vendor/v8 to use local rusty_v8..."
cat > "$VENDOR_DIR/v8/Cargo.toml" << 'EOF'
[package]
name = "v8-wrapper"
version = "152.2.0"
edition = "2024"
publish = false

[features]
default = ["use_custom_libcxx"]
use_custom_libcxx = ["v8-local/use_custom_libcxx"]
v8_enable_pointer_compression = ["v8-local/v8_enable_pointer_compression"]
v8_enable_sandbox = ["v8-local/v8_enable_sandbox"]
v8_enable_v8_checks = ["v8-local/v8_enable_v8_checks"]

[dependencies]
v8-local = { package = "v8", path = "../rusty_v8" }
EOF

cat > "$VENDOR_DIR/v8/src/lib.rs" << 'EOF'
pub use v8_local::*;
EOF
echo "   ✓ Done"

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

# Disable fast-call path in bindings.rs
cat > /tmp/bindings_patch.txt << 'PATCH'
  // Disable fast path for v8 152 - causes SIGILL on RISC-V snapshot creation
  let template = builder.build(scope);
PATCH

# Find and replace the fast_function block (lines ~600-605)
sed -i '/let template = if let Some(fast_function)/,/};/{
  /let template = if let Some(fast_function)/c\
  // Disable fast path for v8 152 - causes SIGILL on RISC-V snapshot creation\
  let template = builder.build(scope);
  /^  }/d
}' "$VENDOR_DIR/deno_core/runtime/bindings.rs"

# Update WasmStreaming generic
sed -i 's/pub struct WasmStreamingResource(pub(crate) RefCell<v8::WasmStreaming>);/pub struct WasmStreamingResource(pub(crate) RefCell<v8::WasmStreaming<false>>);/' "$VENDOR_DIR/deno_core/ops_builtin.rs"

# Update ICU data function
sed -i 's/set_common_data_77/set_common_data_78/' "$VENDOR_DIR/deno_core/runtime/setup.rs"

# Remove --no-validate-asm flag
sed -i 's/" --no-validate-asm",//' "$VENDOR_DIR/deno_core/runtime/setup.rs"

echo "   ✓ Done"

# 8. Update root Cargo.toml
echo "8. Updating root Cargo.toml..."
cd "$REPO_ROOT"

# Add rusty_v8 to workspace exclusions (after members line)
if ! grep -q 'exclude = \["vendor/rusty_v8"\]' Cargo.toml; then
    # Insert exclude after the members array closing bracket
    sed -i '/^members = \[/,/^\]$/s/^\]$/]\nexclude = ["vendor/rusty_v8"]/' Cargo.toml
fi

# Relax ICU pins
sed -i 's/icu_calendar = { version = "=2\.1\.1"/icu_calendar = { version = ">=2.1"/' Cargo.toml
sed -i 's/icu_locale = { version = "=2\.1\.1"/icu_locale = { version = ">=2.1"/' Cargo.toml

# Add patches (before agent-client-protocol)
if ! grep -q 'deno_core = { path = "vendor/deno_core" }' Cargo.toml; then
    sed -i '/\[patch\.crates-io\]/a\
deno_core = { path = "vendor/deno_core" }\
serde_v8 = { path = "vendor/serde_v8" }\
v8 = { package = "v8-wrapper", path = "vendor/v8" }' Cargo.toml
fi

echo "   ✓ Done"

# 9. Update crates/goose/Cargo.toml
echo "9. Updating crates/goose/Cargo.toml..."
sed -i 's/icu_calendar = { version = "=2\.1\.1"/icu_calendar = { version = ">=2.1"/' "$REPO_ROOT/crates/goose/Cargo.toml"
sed -i 's/icu_locale = { version = "=2\.1\.1"/icu_locale = { version = ">=2.1"/' "$REPO_ROOT/crates/goose/Cargo.toml"
echo "   ✓ Done"

# 10. Add RISC-V to update.rs if not already there
echo "10. Adding RISC-V to update command..."
UPDATE_RS="$REPO_ROOT/crates/goose-cli/src/commands/update.rs"
if ! grep -q 'target_arch = "riscv64"' "$UPDATE_RS"; then
    # Find the last } before the closing of asset_name function and add riscv64 case
    cat > /tmp/riscv_case.txt << 'EOF'
    #[cfg(all(target_os = "linux", target_arch = "riscv64"))]
    {
        "goose-riscv64gc-unknown-linux-gnu.tar.bz2"
    }
EOF
    # Insert before the closing brace of asset_name function
    sed -i '/^}$/i\    #[cfg(all(target_os = "linux", target_arch = "riscv64"))]\n    {\n        "goose-riscv64gc-unknown-linux-gnu.tar.bz2"\n    }' "$UPDATE_RS"
fi
echo "   ✓ Done"

# 11. Update dependencies
echo "11. Updating Cargo dependencies..."
cd "$REPO_ROOT"
cargo update --quiet
echo "   ✓ Done"

echo ""
echo "=== Setup complete! ==="
echo ""
echo "To build for RISC-V:"
echo "  cargo build --release --target riscv64gc-unknown-linux-gnu -p goose-cli --bin goose"
echo ""
echo "Output binary:"
echo "  target/riscv64gc-unknown-linux-gnu/release/goose"
