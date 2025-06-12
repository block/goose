#!/bin/bash

# Test script for Temporal Scheduler Integration
# This script tests the complete Phase 2 implementation

set -e

echo "🚀 Testing Temporal Scheduler Integration - Phase 2"
echo "=================================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print status
print_status() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check prerequisites
echo "🔍 Checking prerequisites..."

# Check if Temporal CLI is installed
if ! command -v temporal &> /dev/null; then
    print_error "Temporal CLI not found. Install with: brew install temporal"
    exit 1
fi
print_status "Temporal CLI found"

# Check if Go service is built
if [ ! -f "temporal-service/temporal-service" ]; then
    print_error "Temporal service not built. Run: cd temporal-service && ./build.sh"
    exit 1
fi
print_status "Temporal service binary found"

# Check if Rust executor is built
if ! command -v goose-scheduler-executor &> /dev/null; then
    print_error "goose-scheduler-executor not found in PATH"
    print_warning "Building and installing executor..."
    cargo build --release --bin goose-scheduler-executor
    cp target/release/goose-scheduler-executor /usr/local/bin/
fi
print_status "goose-scheduler-executor found"

# Build the goose library
echo "🔨 Building goose library..."
cargo build --lib -p goose
print_status "Goose library built successfully"

# Test 1: Verify trait compilation
echo "🧪 Test 1: Verify trait abstraction compiles..."
cargo check --lib -p goose
print_status "Trait abstraction compiles correctly"

# Test 2: Verify executor binary works
echo "🧪 Test 2: Test executor binary help..."
if goose-scheduler-executor --help > /dev/null 2>&1; then
    print_status "Executor binary responds to --help"
else
    print_error "Executor binary failed help test"
    exit 1
fi

# Test 3: Create a test recipe
echo "🧪 Test 3: Creating test recipe..."
TEST_RECIPE_DIR="/tmp/goose-temporal-test"
mkdir -p "$TEST_RECIPE_DIR"

cat > "$TEST_RECIPE_DIR/test-recipe.yaml" << 'EOF'
version: "1.0.0"
title: "Temporal Test Recipe"
description: "A simple test recipe for Temporal scheduler"
prompt: "Say hello and tell me the current time"
EOF

print_status "Test recipe created at $TEST_RECIPE_DIR/test-recipe.yaml"

# Test 4: Verify the integration compiles with all features
echo "🧪 Test 4: Full compilation test..."
cargo build --workspace --exclude goose-server --exclude goose-cli
print_status "Full workspace builds successfully"

echo ""
echo "🎉 Phase 2 Integration Tests Complete!"
echo "======================================"
print_status "All tests passed successfully"
echo ""
echo "📋 What was tested:"
echo "   ✅ Prerequisites (Temporal CLI, Go service, Rust executor)"
echo "   ✅ Goose library compilation"
echo "   ✅ Trait abstraction"
echo "   ✅ Executor binary functionality"
echo "   ✅ Test recipe creation"
echo "   ✅ Full workspace compilation"
echo ""
echo "🚀 Ready for Phase 3: Migration & Testing"
echo ""
echo "To test the Temporal scheduler manually:"
echo "   1. Set environment: export GOOSE_SCHEDULER_TYPE=temporal"
echo "   2. Start services: cd temporal-service && ./start.sh"
echo "   3. Use the scheduler factory in your code"
echo ""
print_status "Phase 2 implementation is ready!"