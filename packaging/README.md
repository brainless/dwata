# Packaging Files

This directory contains distribution-specific packaging files for dwata.

## Available Packages

### Arch Linux (`arch/`)
- **PKGBUILD**: Build recipe for Arch Linux and AUR
- **INSTALL.md**: Installation instructions for Arch users

See [arch/INSTALL.md](arch/INSTALL.md) for installation options.

## Binary Releases

For other distributions, pre-built binaries are available in [GitHub Releases](https://github.com/brainless/dwata/releases):

- **Linux (static)**: `dwata-api-{version}-linux-x64-static.tar.gz` - Works on any Linux distro
- **Linux (dynamic)**: `dwata-api-{version}-linux-x64.tar.gz` - Standard dynamically-linked binary
- **Debian/Ubuntu**: `dwata-api-{version}-linux-x64.deb`
- **RHEL/Fedora**: `dwata-api-{version}-linux-x64.rpm`
- **macOS**: `dwata-api-{version}-macos-x64.tar.gz`
- **Windows**: `dwata-api-{version}-windows-x64.zip`

## Contributing

To add packaging for additional distributions, create a new subdirectory with the appropriate build files and documentation.
