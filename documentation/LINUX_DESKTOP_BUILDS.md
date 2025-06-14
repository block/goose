# Linux Desktop Builds

This document describes the Linux desktop build pipeline for Goose, which creates both DEB and RPM packages for distribution.

## Overview

The Linux desktop build pipeline creates native Linux packages (.deb and .rpm) that include:
- Goose Desktop Application with GUI
- Built-in goosed server binary
- Desktop integration and menu entries
- Support for x64 architecture

## Build Pipeline

### Automated Builds

The Linux desktop builds are automatically triggered in the following scenarios:

1. **Release Builds** (`release.yml`): When a version tag is pushed (e.g., `v1.0.0`)
2. **Canary Builds** (`canary.yml`): On every push to the `main` branch
3. **PR Testing** (`pr-comment-bundle-linux.yml`): When a PR comment contains `/bundle-linux`

### Manual Testing

To test Linux builds on a Pull Request:

1. Comment `/bundle-linux` on any PR
2. The build will be triggered automatically
3. Download links will be posted as a comment once the build completes

## Package Formats

### DEB Package (Ubuntu/Debian)
- **File**: `goose_<version>_amd64.deb`
- **Architecture**: x64 (amd64)
- **Compatible with**: Ubuntu, Debian, and derivatives

### RPM Package (RHEL/Fedora/CentOS)
- **File**: `goose-<version>.x86_64.rpm`
- **Architecture**: x64 (x86_64)
- **Compatible with**: RHEL, Fedora, CentOS, and derivatives

## Installation

### Ubuntu/Debian (.deb)

```bash
# Download the package
wget https://github.com/block/goose/releases/latest/download/goose_<version>_amd64.deb

# Install the package
sudo dpkg -i goose_<version>_amd64.deb

# Fix any dependency issues
sudo apt-get install -f
```

### RHEL/Fedora/CentOS (.rpm)

```bash
# Download the package
wget https://github.com/block/goose/releases/latest/download/goose-<version>.x86_64.rpm

# Install with yum (RHEL/CentOS)
sudo yum install goose-<version>.x86_64.rpm

# Or install with dnf (Fedora)
sudo dnf install goose-<version>.x86_64.rpm

# Or install with rpm directly
sudo rpm -i goose-<version>.x86_64.rpm
```

## Build Process

The Linux build process includes:

1. **System Dependencies**: Install required Linux packaging tools (rpm, fakeroot, dpkg-dev)
2. **Rust Build**: Compile the goosed server binary
3. **Electron Build**: Package the desktop application with Electron Forge
4. **Package Generation**: Create both .deb and .rpm packages
5. **Artifact Upload**: Upload packages as GitHub Actions artifacts

## System Requirements

### Build Environment
- Ubuntu Latest (GitHub Actions runner)
- Node.js 23
- Rust stable toolchain
- System dependencies for Linux packaging

### Runtime Requirements
- Linux x64 (amd64/x86_64)
- Standard Linux desktop environment
- System libraries for Electron applications

## Troubleshooting

### Common Build Issues

1. **Disk Space**: The build process requires significant disk space. Aggressive cleanup is performed during the build.
2. **Dependencies**: Missing system dependencies can cause build failures. The pipeline installs all required packages.
3. **Electron Packaging**: Issues with Electron Forge configuration can prevent package generation.

### Package Installation Issues

1. **Missing Dependencies**: Use `apt-get install -f` (DEB) or package manager dependency resolution
2. **Architecture Mismatch**: Ensure you're installing the correct package for your architecture
3. **Permission Issues**: Package installation requires root/sudo privileges

## Development

### Local Testing

To test the Linux build locally:

```bash
# Install system dependencies (Ubuntu/Debian)
sudo apt-get install build-essential libnss3-dev libatk-bridge2.0-dev \
  libdrm2 libxcomposite1 libxdamage1 libxrandr2 libgbm1 libxss1 \
  libasound2t64 rpm fakeroot dpkg-dev protobuf-compiler

# Build the Rust binary
cargo build --release -p goose-server

# Copy binary to Electron folder
mkdir -p ui/desktop/src/bin
cp target/release/goosed ui/desktop/src/bin/

# Build Electron packages
cd ui/desktop
npm install
npm run make -- --platform=linux --arch=x64
```

### Modifying the Build

The Linux build configuration is defined in:
- `.github/workflows/bundle-desktop-linux.yml` - Main build workflow
- `.github/workflows/pr-comment-bundle-linux.yml` - PR testing workflow
- `ui/desktop/forge.config.js` - Electron Forge configuration

## Artifacts

Build artifacts are uploaded to GitHub Actions and include:
- `Goose-linux-x64-deb.zip` - DEB package
- `Goose-linux-x64-rpm.zip` - RPM package  
- `Goose-linux-x64.zip` - Combined package with both formats

## Support

For issues with Linux builds or packages:
1. Check the [GitHub Actions logs](https://github.com/block/goose/actions)
2. Review this documentation
3. Open an issue in the Goose repository
4. Contact the development team