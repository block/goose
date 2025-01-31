# Justfile

# list all tasks
default:
  @just --list

# Default release command
release-binary:
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
run-ui:
    @just release-binary
    @echo "Running UI..."
    cd ui/desktop && npm install && npm run start-gui

# Run Docusaurus server for documentation
run-docs:
    @echo "Running docs server..."
    cd documentation && yarn && yarn start

# Run server
run-server:
    @echo "Running server..."
    cargo run -p goose-server

# make GUI with latest binary
make-ui:
    @just release-binary
    cd ui/desktop && npm run bundle:default

# Setup langfuse server
langfuse-server:
    #!/usr/bin/env bash
    ./scripts/setup_langfuse.sh

# ensure the current branch is "main" or error
ensure-main:
    #!/usr/bin/env bash
    branch=$(git rev-parse --abbrev-ref HEAD); \
    if [ "$branch" != "main" ]; then \
        echo "Error: You are not on the main branch (current: $branch)"; \
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
release version: ensure-main
    @just validate {{ version }} || exit 1

    @git switch -c "release/{{ version }}"
    @uvx --from=toml-cli toml set --toml-path=Cargo.toml "workspace.package.version" {{ version }}

    @cd ui/desktop && npm version {{ version }} --no-git-tag-version --allow-same-version

    @git add Cargo.toml ui/desktop/package.json ui/desktop/package-lock.json
    @git commit --message "chore(release): release version {{ version }}"

# extract version from Cargo.toml
get-tag-version:
    @uvx --from=toml-cli toml get --toml-path=Cargo.toml "workspace.package.version"

# create the git tag from Cargo.toml, must be on main
tag: ensure-main
    echo git tag v$(just get-tag-version)

# create tag and push to origin (use this when release branch is merged to main)
tag-push: tag
    # this will kick of ci for release
    echo git push origin tag v$(just get-tag-version)

# generate release notes from git commits
release-notes:
    #!/usr/bin/env bash
    git log --pretty=format:"- %s" v$(just get-tag-version)..HEAD
