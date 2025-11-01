# Deployment Guide

This document describes how to deploy and release `importee` to various distribution channels.

## Table of Contents

- [PyPI Deployment](#pypi-deployment)
- [Snap Store Deployment](#snap-store-deployment)
- [GitHub Releases](#github-releases)
- [Development Builds](#development-builds)

## PyPI Deployment

### Prerequisites

1. **PyPI Account**: Create an account at [pypi.org](https://pypi.org)
2. **Trusted Publishing Setup**: Configure GitHub Actions trusted publishing:
   - Go to PyPI → Account Settings → Publishing
   - Add a new publisher:
     - PyPI project name: `importee`
     - Owner: `your-github-username`
     - Repository: `importee`
     - Workflow: `publish.yml`
     - Environment: `pypi`

### Release Process

1. **Update Version**: Update version in both files:
   ```toml
   # pyproject.toml
   [project]
   version = "0.2.0"
   
   # Cargo.toml
   [package]
   version = "0.2.0"
   ```

2. **Commit and Tag**:
   ```bash
   git add pyproject.toml Cargo.toml
   git commit -m "Bump version to 0.2.0"
   git tag v0.2.0
   git push origin main --tags
   ```

3. **Create GitHub Release**:
   - Go to GitHub → Releases → Create new release
   - Choose the tag you just created
   - Add release notes describing changes
   - Click "Publish release"

4. **Automatic Publishing**: 
   - The GitHub Action will automatically build wheels for Linux, macOS, and Windows
   - It will publish to PyPI using trusted publishing (no API key needed)
   - Monitor progress at: `https://github.com/yourusername/importee/actions`

### Manual Publishing (Alternative)

If you need to publish manually:

```bash
# Install maturin
pip install maturin

# Build wheels
maturin build --release

# Publish to PyPI
maturin publish
```

You'll need to set up a PyPI API token and configure it:
```bash
export MATURIN_PYPI_TOKEN=pypi-...
```

## Snap Store Deployment

### Prerequisites

1. **Snapcraft Account**: Create an account at [snapcraft.io](https://snapcraft.io)
2. **Install Snapcraft**:
   ```bash
   sudo snap install snapcraft --classic
   ```

### Building Locally

```bash
# Build the snap
snapcraft

# Test the snap locally
sudo snap install ./importee_0.1.0_amd64.snap --dangerous

# Try it out
importee check
```

### Publishing to Snap Store

1. **Login to Snapcraft**:
   ```bash
   snapcraft login
   ```

2. **Register the Snap Name** (first time only):
   ```bash
   snapcraft register importee
   ```

3. **Build and Upload**:
   ```bash
   # Build
   snapcraft
   
   # Upload to edge channel (for testing)
   snapcraft upload --release=edge importee_*.snap
   
   # Promote to stable when ready
   snapcraft promote importee --from-channel edge --to-channel stable
   ```

### Automated Snap Publishing

You can add a GitHub Action to automate snap building:

```yaml
# .github/workflows/snap.yml
name: Build Snap

on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Build snap
        uses: snapcore/action-build@v1
      
      - name: Publish snap
        uses: snapcore/action-publish@v1
        with:
          snap: importee_*.snap
          release: stable
        env:
          SNAPCRAFT_STORE_CREDENTIALS: ${{ secrets.SNAPCRAFT_TOKEN }}
```

To set this up:
1. Export your snapcraft credentials:
   ```bash
   snapcraft export-login --snaps=importee --channels=stable,candidate,beta,edge snapcraft-token.txt
   ```
2. Add the token as a GitHub secret named `SNAPCRAFT_TOKEN`

## GitHub Releases

Releases are automatically created when you push a tag. To customize release notes:

1. **Push a Tag**:
   ```bash
   git tag -a v0.2.0 -m "Release version 0.2.0"
   git push origin v0.2.0
   ```

2. **Create Release on GitHub**:
   - Go to Releases → Draft a new release
   - Select the tag
   - Add release notes (use the template below)
   - Attach any additional binaries if needed

### Release Notes Template

```markdown
## Changes

- List actual changes made
- Bug fixes if any
- New features if any

## Installation

pip install importee

**Full Changelog**: https://github.com/yourusername/importee/compare/v0.1.0...v0.2.0
```

## Development Builds

### Local Development

```bash
# Build and install in development mode
maturin develop

# Or use the Makefile
make dev
```

### Testing Release Builds

```bash
# Build release wheels locally
maturin build --release

# Test the wheel
pip install target/wheels/importee-*.whl
```

## CI/CD Pipeline

The project uses GitHub Actions for CI/CD:

- **`ci.yml`**: Runs on every push and PR
  - Tests on Python 3.9, 3.10, 3.11, 3.12
  - Tests on Linux, macOS, Windows
  - Runs linting (Rust and Python)
  - Builds wheels and source distribution

- **`publish.yml`**: Runs on releases
  - Builds wheels for all platforms
  - Publishes to PyPI using trusted publishing

## Platform Support

### Wheels Built

| Platform | Python Versions |
|----------|----------------|
| Linux (x86_64) | 3.9, 3.10, 3.11, 3.12 |
| macOS (x86_64, ARM64) | 3.9, 3.10, 3.11, 3.12 |
| Windows (x86_64) | 3.9, 3.10, 3.11, 3.12 |

### Snap

- Available for: `amd64`, `arm64`, `armhf`, `i386`
- Base: `core22` (Ubuntu 22.04)
- Confinement: `strict`

## Troubleshooting

### Maturin Build Issues

If you encounter build issues:

```bash
# Clean and rebuild
cargo clean
maturin develop --release
```

### PyPI Upload Failures

1. Check that trusted publishing is configured correctly
2. Verify the workflow has `id-token: write` permission
3. Ensure the environment name matches (`pypi`)

### Snap Build Issues

```bash
# Clean snap build
snapcraft clean

# Rebuild
snapcraft --debug
```

## Release Checklist

Before releasing a new version:

- [ ] Update version in `pyproject.toml` and `Cargo.toml`
- [ ] Update `CHANGELOG.md` with changes
- [ ] Run tests locally: `pytest tests/`
- [ ] Test build locally: `maturin build --release`
- [ ] Create and push git tag
- [ ] Create GitHub release with notes
- [ ] Verify CI/CD pipeline succeeds
- [ ] Test installation from PyPI: `pip install importee==X.Y.Z`
- [ ] Build and test snap locally
- [ ] Publish snap to store

## Questions?

For issues or questions about deployment, please open an issue on GitHub.

