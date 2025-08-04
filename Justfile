# Justfile

# list all tasks
default:
  @just --list

# Default release command
release-binary:
    @echo "Building release version..."
    cargo build --release
    @just copy-binary
    @echo "Generating OpenAPI schema..."
    cargo run -p goose-server --bin generate_schema

# Build Windows executable
release-windows:
    #!/usr/bin/env sh
    if [ "$(uname)" = "Darwin" ] || [ "$(uname)" = "Linux" ]; then
        echo "Building Windows executable using Docker..."
        docker volume create goose-windows-cache || true
        docker run --rm \
            -v "$(pwd)":/usr/src/myapp \
            -v goose-windows-cache:/usr/local/cargo/registry \
            -w /usr/src/myapp \
            rust:latest \
            sh -c "rustup target add x86_64-pc-windows-gnu && \
                apt-get update && \
                apt-get install -y mingw-w64 protobuf-compiler cmake && \
                export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc && \
                export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++ && \
                export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar && \
                export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc && \
                export PKG_CONFIG_ALLOW_CROSS=1 && \
                export PROTOC=/usr/bin/protoc && \
                export PATH=/usr/bin:\$PATH && \
                protoc --version && \
                cargo build --release --target x86_64-pc-windows-gnu && \
                GCC_DIR=\$(ls -d /usr/lib/gcc/x86_64-w64-mingw32/*/ | head -n 1) && \
                cp \$GCC_DIR/libstdc++-6.dll /usr/src/myapp/target/x86_64-pc-windows-gnu/release/ && \
                cp \$GCC_DIR/libgcc_s_seh-1.dll /usr/src/myapp/target/x86_64-pc-windows-gnu/release/ && \
                cp /usr/x86_64-w64-mingw32/lib/libwinpthread-1.dll /usr/src/myapp/target/x86_64-pc-windows-gnu/release/"
    else
        echo "Building Windows executable using Docker through PowerShell..."
        powershell.exe -Command "docker volume create goose-windows-cache; docker run --rm -v ${PWD}:/usr/src/myapp -v goose-windows-cache:/usr/local/cargo/registry -w /usr/src/myapp rust:latest sh -c 'rustup target add x86_64-pc-windows-gnu && apt-get update && apt-get install -y mingw-w64 && cargo build --release --target x86_64-pc-windows-gnu && GCC_DIR=\$(ls -d /usr/lib/gcc/x86_64-w64-mingw32/*/ | head -n 1) && cp \$GCC_DIR/libstdc++-6.dll /usr/src/myapp/target/x86_64-pc-windows-gnu/release/ && cp \$GCC_DIR/libgcc_s_seh-1.dll /usr/src/myapp/target/x86_64-pc-windows-gnu/release/ && cp /usr/x86_64-w64-mingw32/lib/libwinpthread-1.dll /usr/src/myapp/target/x86_64-pc-windows-gnu/release/'"
    fi
    echo "Windows executable and required DLLs created at ./target/x86_64-pc-windows-gnu/release/"

# Build for Intel Mac
release-intel:
    @echo "Building release version for Intel Mac..."
    cargo build --release --target x86_64-apple-darwin
    @just copy-binary-intel

copy-binary BUILD_MODE="release":
    @if [ -f ./target/{{BUILD_MODE}}/goosed ]; then \
        echo "Copying goosed binary from target/{{BUILD_MODE}}..."; \
        cp -p ./target/{{BUILD_MODE}}/goosed ./ui/desktop/src/bin/; \
    else \
        echo "Binary not found in target/{{BUILD_MODE}}"; \
        exit 1; \
    fi
    @if [ -f ./target/{{BUILD_MODE}}/goose ]; then \
        echo "Copying goose CLI binary from target/{{BUILD_MODE}}..."; \
        cp -p ./target/{{BUILD_MODE}}/goose ./ui/desktop/src/bin/; \
    else \
        echo "Goose CLI binary not found in target/{{BUILD_MODE}}"; \
        exit 1; \
    fi
    @if [ -f ./temporal-service/temporal-service ]; then \
        echo "Copying temporal-service binary..."; \
        cp -p ./temporal-service/temporal-service ./ui/desktop/src/bin/; \
    else \
        echo "temporal-service binary not found. Building it..."; \
        cd temporal-service && ./build.sh && cp -p temporal-service ../ui/desktop/src/bin/; \
    fi
    @echo "Checking temporal CLI binary..."
    @if [ ! -f ./ui/desktop/src/bin/temporal ]; then \
        echo "temporal CLI binary not found in ui/desktop/src/bin/"; \
        echo "Please ensure temporal CLI is available or will be downloaded at runtime"; \
    else \
        echo "temporal CLI binary found"; \
    fi

# Copy binary command for Intel build
copy-binary-intel:
    @if [ -f ./target/x86_64-apple-darwin/release/goosed ]; then \
        echo "Copying Intel goosed binary to ui/desktop/src/bin with permissions preserved..."; \
        cp -p ./target/x86_64-apple-darwin/release/goosed ./ui/desktop/src/bin/; \
    else \
        echo "Intel release binary not found."; \
        exit 1; \
    fi
    @if [ -f ./target/x86_64-apple-darwin/release/goose ]; then \
        echo "Copying Intel goose CLI binary to ui/desktop/src/bin..."; \
        cp -p ./target/x86_64-apple-darwin/release/goose ./ui/desktop/src/bin/; \
    else \
        echo "Intel goose CLI binary not found."; \
        exit 1; \
    fi
    @if [ -f ./temporal-service/temporal-service ]; then \
        echo "Copying temporal-service binary..."; \
        cp -p ./temporal-service/temporal-service ./ui/desktop/src/bin/; \
    else \
        echo "temporal-service binary not found. Building it..."; \
        cd temporal-service && ./build.sh && cp -p temporal-service ../ui/desktop/src/bin/; \
    fi
    @echo "Checking temporal CLI binary..."
    @if [ ! -f ./ui/desktop/src/bin/temporal ]; then \
        echo "temporal CLI binary not found in ui/desktop/src/bin/"; \
        echo "Please ensure temporal CLI is available or will be downloaded at runtime"; \
    else \
        echo "temporal CLI binary found"; \
    fi

# Copy Windows binary command
copy-binary-windows:
    @powershell.exe -Command "if (Test-Path ./target/x86_64-pc-windows-gnu/release/goosed.exe) { \
        Write-Host 'Copying Windows binary and DLLs to ui/desktop/src/bin...'; \
        Copy-Item -Path './target/x86_64-pc-windows-gnu/release/goosed.exe' -Destination './ui/desktop/src/bin/' -Force; \
        Copy-Item -Path './target/x86_64-pc-windows-gnu/release/*.dll' -Destination './ui/desktop/src/bin/' -Force; \
    } else { \
        Write-Host 'Windows binary not found.' -ForegroundColor Red; \
        exit 1; \
    }"
    @powershell.exe -Command "if (Test-Path ./target/x86_64-pc-windows-gnu/release/goose-scheduler-executor.exe) { \
        Write-Host 'Copying Windows goose-scheduler-executor binary...'; \
        Copy-Item -Path './target/x86_64-pc-windows-gnu/release/goose-scheduler-executor.exe' -Destination './ui/desktop/src/bin/' -Force; \
    } else { \
        Write-Host 'Windows goose-scheduler-executor binary not found.' -ForegroundColor Yellow; \
    }"
    @if [ -f ./temporal-service/temporal-service.exe ]; then \
        echo "Copying Windows temporal-service binary..."; \
        cp -p ./temporal-service/temporal-service.exe ./ui/desktop/src/bin/; \
    else \
        echo "Windows temporal-service binary not found. Building it..."; \
        cd temporal-service && GOOS=windows GOARCH=amd64 go build -o temporal-service.exe main.go && cp temporal-service.exe ../ui/desktop/src/bin/; \
    fi
    @echo "Note: Temporal CLI for Windows will be downloaded at runtime if needed"

# Run UI with latest
run-ui:
    @just release-binary
    @echo "Running UI..."
    cd ui/desktop && npm install && npm run start-gui

run-ui-only:
    @echo "Running UI..."
    cd ui/desktop && npm install && npm run start-gui

debug-ui:
	@echo "🚀 Starting Goose frontend in external backend mode"
	cd ui/desktop && \
	export GOOSE_EXTERNAL_BACKEND=true && \
	export GOOSE_EXTERNAL_PORT=3000 && \
	npm install && \
	npm run start-gui

# Run UI with main process debugging enabled
# To debug main process:
# 1. Run: just debug-ui-main-process
# 2. Open Chrome → chrome://inspect
# 3. Click "Open dedicated DevTools for Node"
# 4. If not auto-detected, click "Configure" and add: localhost:9229

debug-ui-main-process:
	@echo "🔍 Starting Goose UI with main process debugging enabled"
	@just release-binary
	cd ui/desktop && \
	npm install && \
	npm run start-gui-debug

# Run UI with alpha changes
run-ui-alpha temporal="true":
    @just release-binary
    @echo "Running UI with {{ if temporal == "true" { "Temporal" } else { "Legacy" } }} scheduler..."
    cd ui/desktop && npm install && ALPHA=true GOOSE_SCHEDULER_TYPE={{ if temporal == "true" { "temporal" } else { "legacy" } }} npm run start-alpha-gui

# Run UI with alpha changes using legacy scheduler (no Temporal dependency)
run-ui-alpha-legacy:
    @just release-binary
    @echo "Running UI with Legacy scheduler (no Temporal required)..."
    cd ui/desktop && npm install && ALPHA=true GOOSE_SCHEDULER_TYPE=legacy npm run start-alpha-gui

# Run UI with latest (Windows version)
run-ui-windows:
    @just release-windows
    @powershell.exe -Command "Write-Host 'Copying Windows binary...'"
    @just copy-binary-windows
    @powershell.exe -Command "Write-Host 'Running UI...'; Set-Location ui/desktop; npm install; npm run start-gui"

# Run Docusaurus server for documentation
run-docs:
    @echo "Running docs server..."
    cd documentation && yarn && yarn start

# Run server
run-server:
    @echo "Running server..."
    cargo run -p goose-server

# Check if OpenAPI schema is up-to-date
check-openapi-schema: generate-openapi
    ./scripts/check-openapi-schema.sh

# Generate OpenAPI specification without starting the UI
generate-openapi:
    @echo "Generating OpenAPI schema..."
    cargo run -p goose-server --bin generate_schema
    @echo "Generating frontend API..."
    cd ui/desktop && npm run generate-api

# make GUI with latest binary
lint-ui:
    cd ui/desktop && npm run lint:check

# make GUI with latest binary
make-ui:
    @just release-binary
    cd ui/desktop && npm run bundle:default

# make GUI with latest binary and alpha features enabled
make-ui-alpha:
    @just release-binary
    cd ui/desktop && npm run bundle:alpha

# make GUI with latest Windows binary
make-ui-windows:
    @just release-windows
    #!/usr/bin/env sh
    set -e
    if [ -f "./target/x86_64-pc-windows-gnu/release/goosed.exe" ]; then \
        echo "Cleaning destination directory..." && \
        rm -rf ./ui/desktop/src/bin && \
        mkdir -p ./ui/desktop/src/bin && \
        echo "Copying Windows binary and DLLs..." && \
        cp -f ./target/x86_64-pc-windows-gnu/release/goosed.exe ./ui/desktop/src/bin/ && \
        cp -f ./target/x86_64-pc-windows-gnu/release/*.dll ./ui/desktop/src/bin/ && \
        echo "Starting Windows package build..." && \
        (cd ui/desktop && npm run bundle:windows) && \
        echo "Windows package build complete!"; \
    else \
        echo "Windows binary not found."; \
        exit 1; \
    fi

# make GUI with latest binary
make-ui-intel:
    @just release-intel
    cd ui/desktop && npm run bundle:intel

# Start Temporal services (server and temporal-service)
start-temporal:
    @echo "Starting Temporal server..."
    @if ! pgrep -f "temporal server start-dev" > /dev/null; then \
        echo "Starting Temporal server in background..."; \
        nohup temporal server start-dev --db-filename temporal.db --port 7233 --ui-port 8233 --log-level warn > temporal-server.log 2>&1 & \
        echo "Waiting for Temporal server to start..."; \
        sleep 5; \
    else \
        echo "Temporal server is already running"; \
    fi
    @echo "Starting temporal-service..."
    @if ! pgrep -f "temporal-service" > /dev/null; then \
        echo "Starting temporal-service in background..."; \
        cd temporal-service && nohup ./temporal-service > temporal-service.log 2>&1 & \
        echo "Waiting for temporal-service to start..."; \
        sleep 3; \
    else \
        echo "temporal-service is already running"; \
    fi
    @echo "Temporal services started. Check logs: temporal-server.log, temporal-service/temporal-service.log"

# Stop Temporal services
stop-temporal:
    @echo "Stopping Temporal services..."
    @pkill -f "temporal server start-dev" || echo "Temporal server was not running"
    @pkill -f "temporal-service" || echo "temporal-service was not running"
    @echo "Temporal services stopped"

# Check status of Temporal services
status-temporal:
    @echo "Checking Temporal services status..."
    @if pgrep -f "temporal server start-dev" > /dev/null; then \
        echo "✓ Temporal server is running"; \
    else \
        echo "✗ Temporal server is not running"; \
    fi
    @if pgrep -f "temporal-service" > /dev/null; then \
        echo "✓ temporal-service is running"; \
    else \
        echo "✗ temporal-service is not running"; \
    fi
    @echo "Testing temporal-service health..."
    @curl -s http://localhost:8080/health > /dev/null && echo "✓ temporal-service is responding" || echo "✗ temporal-service is not responding"

# Run UI with debug build
run-dev:
    @echo "Building development version..."
    cargo build
    @just copy-binary debug
    @echo "Running UI..."
    cd ui/desktop && npm run start-gui

# Install all dependencies (run once after fresh clone)
install-deps:
    cd ui/desktop && npm install
    cd documentation && yarn

ensure-release-branch:
    #!/usr/bin/env bash
    branch=$(git rev-parse --abbrev-ref HEAD); \
    if [[ ! "$branch" == release/* ]]; then \
        echo "Error: You are not on a release branch (current: $branch)"; \
        exit 1; \
    fi

    # check that main is up to date with upstream main
    git fetch
    # @{u} refers to upstream branch of current branch
    if [ "$(git rev-parse HEAD)" != "$(git rev-parse @{u})" ]; then \
        echo "Error: Your branch is not up to date with the upstream branch"; \
        echo "  ensure your branch is up to date (git pull)"; \
        exit 1; \
    fi

# validate the version is semver, and not the current version
validate version:
    #!/usr/bin/env bash
    if [[ ! "{{ version }}" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-.*)?$ ]]; then
      echo "[error]: invalid version '{{ version }}'."
      echo "  expected: semver format major.minor.patch or major.minor.patch-<suffix>"
      exit 1
    fi

    current_version=$(just get-tag-version)
    if [[ "{{ version }}" == "$current_version" ]]; then
      echo "[error]: current_version '$current_version' is the same as target version '{{ version }}'"
      echo "  expected: new version in semver format"
      exit 1
    fi

# set cargo and app versions, must be semver
prepare-release version:
    @just validate {{ version }} || exit 1

    @git switch -c "release/{{ version }}"
    @uvx --from=toml-cli toml set --toml-path=Cargo.toml "workspace.package.version" {{ version }}

    @cd ui/desktop && npm version {{ version }} --no-git-tag-version --allow-same-version

    # see --workspace flag https://doc.rust-lang.org/cargo/commands/cargo-update.html
    # used to update Cargo.lock after we've bumped versions in Cargo.toml
    @cargo update --workspace
    @git add Cargo.toml Cargo.lock ui/desktop/package.json ui/desktop/package-lock.json
    @git commit --message "chore(release): release version {{ version }}"

# extract version from Cargo.toml
get-tag-version:
    @uvx --from=toml-cli toml get --toml-path=Cargo.toml "workspace.package.version"

# create the git tag from Cargo.toml, checking we're on a release branch
tag: ensure-release-branch
    git tag v$(just get-tag-version)

# create tag and push to origin (use this when release branch is merged to main)
tag-push: tag
    # this will kick of ci for release
    git push origin tag v$(just get-tag-version)

# generate release notes from git commits
release-notes old:
    #!/usr/bin/env bash
    git log --pretty=format:"- %s" {{ old }}..v$(just get-tag-version)

### s = file seperator based on OS
s := if os() == "windows" { "\\" } else { "/" }

### testing/debugging
os:
  echo "{{os()}}"
  echo "{{s}}"

# Make just work on Window
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

### Build the core code
### profile = --release or "" for debug
### allparam = OR/AND/ANY/NONE --workspace --all-features --all-targets
win-bld profile allparam:
  cargo run {{profile}} -p goose-server --bin  generate_schema
  cargo build {{profile}} {{allparam}}

### Build just debug
win-bld-dbg:
  just win-bld " " " "

### Build debug and test, examples,...
win-bld-dbg-all:
  just win-bld " " "--workspace --all-targets --all-features"

### Build just release
win-bld-rls:
  just win-bld "--release" " "

### Build release and test, examples, ...
win-bld-rls-all:
  just win-bld "--release" "--workspace --all-targets --all-features"

### Install npm stuff
win-app-deps:
  cd ui{{s}}desktop ; npm install

### Windows copy {release|debug} files to ui\desktop\src\bin
### s = os depenent file seperator
### profile = release or debug
win-copy-win profile:
  copy target{{s}}{{profile}}{{s}}*.exe ui{{s}}desktop{{s}}src{{s}}bin
  copy target{{s}}{{profile}}{{s}}*.dll ui{{s}}desktop{{s}}src{{s}}bin

### "Other" copy {release|debug} files to ui/desktop/src/bin
### s = os depenent file seperator
### profile = release or debug
win-copy-oth profile:
  find target{{s}}{{profile}}{{s}} -maxdepth 1 -type f -executable -print -exec cp {} ui{{s}}desktop{{s}}src{{s}}bin \;

### copy files depending on OS
### profile = release or debug
win-app-copy profile="release":
  just win-copy-{{ if os() == "windows" { "win" } else { "oth" } }} {{profile}}

### Only copy binaries, npm install, start-gui
### profile = release or debug
### s = os depenent file seperator
win-app-run profile:
  just win-app-copy {{profile}}
  just win-app-deps
  cd ui{{s}}desktop ; npm run start-gui

### Only run debug desktop, no build
win-run-dbg:
  just win-app-run "debug"

### Only run release desktop, nu build
win-run-rls:
  just win-app-run "release"

### Build and run debug desktop. tot = cli and desktop
### allparam = nothing or -all passed on command line
### -all = build with --workspace --all-targets --all-features
win-total-dbg *allparam:
  just win-bld-dbg{{allparam}}
  just win-run-dbg

### Build and run release desktop
### allparam = nothing or -all passed on command line
### -all = build with --workspace --all-targets --all-features
win-total-rls *allparam:
  just win-bld-rls{{allparam}}
  just win-run-rls

