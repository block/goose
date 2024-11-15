# Download tokenizer models and provide common cargo commands
set dotenv-load := true
set shell := ["bash", "-c"]

# List all available commands
default:
    @just --list

# === Tokenizer Download Commands ===

# Download all tokenizer files
download-all-tokenizers: (download-gpt4) (download-claude) (download-qwen)

# Download GPT-4 tokenizer
download-gpt4:
    #!/usr/bin/env bash
    mkdir -p tokenizer_files/Xenova--gpt-4o
    curl -L "https://huggingface.co/Xenova/gpt-4o/resolve/main/tokenizer.json" \
        -o "tokenizer_files/Xenova--gpt-4o/tokenizer.json"

# Download Claude tokenizer
download-claude:
    #!/usr/bin/env bash
    mkdir -p tokenizer_files/Xenova--claude-tokenizer
    curl -L "https://huggingface.co/Xenova/claude-tokenizer/resolve/main/tokenizer.json" \
        -o "tokenizer_files/Xenova--claude-tokenizer/tokenizer.json"

# Download Qwen tokenizer
download-qwen:
    #!/usr/bin/env bash
    mkdir -p "tokenizer_files/Qwen--Qwen2.5-Coder-32B-Instruct"
    curl -L "https://huggingface.co/Qwen/Qwen2.5-Coder-32B-Instruct/resolve/main/tokenizer.json" \
        -o "tokenizer_files/Qwen--Qwen2.5-Coder-32B-Instruct/tokenizer.json"

# === Cargo Commands ===

# Build the project
build:
    cargo build

# Build with optimizations
release:
    cargo build --release

# Run the project
run:
    cargo run

# Run with optimizations
run-release:
    cargo run --release

# Run tests
test:
    cargo test

# Check the project for errors
check:
    cargo check

# Format the code
fmt:
    cargo fmt

# Run clippy lints
clippy:
    cargo clippy -- -D warnings

# Generate documentation
doc:
    cargo doc --no-deps --document-private-items

# Open documentation in browser
doc-open: doc
    cargo doc --open

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and run tests
watch-test:
    cargo watch -x test

# Watch for changes and run the project
watch:
    cargo watch -x run

# Update dependencies
update:
    cargo update

# Check for outdated dependencies
outdated:
    cargo outdated

# Run all checks (format, clippy, test)
check-all: fmt clippy test
    @echo "All checks passed!"