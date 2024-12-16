# Justfile

# Default release command
release:
    @echo "Building release version..."
    cargo build --release
    @just copy-binary

# Copy binary command
copy-binary:
    @if [ -f ./target/release/goosed ]; then \
        echo "Copying goosed binary to ui/desktop/src/bin with permissions preserved..."; \
        cp -p ./target/release/goosed ./ui/desktop/src/bin/; \
    else \
        echo "Release binary not found."; \
        exit 1; \
    fi
# Run UI with latest
run-ui: download-tokenizers
    @just release
    @echo "Running UI..."
    cd ui/desktop && npm install && npm run start-gui
    
# Run server
run-server:
    @echo "Running server..."
    cargo run -p goose-server

# Download tokenizer files if they don't exist
download-tokenizers:
    @echo "Checking and downloading tokenizer files..."
    ./download_tokenizers.sh