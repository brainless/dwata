# Installing dwata on Arch Linux

## Option 1: Using the Static Binary (Easiest)

Download the static binary from the latest release:

```bash
# Download the static binary
wget https://github.com/brainless/dwata/releases/latest/download/dwata-api-<VERSION>-linux-x64-static.tar.gz

# Extract
tar -xzf dwata-api-<VERSION>-linux-x64-static.tar.gz

# Move to a location in your PATH
sudo mv dwata-api /usr/local/bin/

# Make executable
sudo chmod +x /usr/local/bin/dwata-api

# Run
dwata-api
```

## Option 2: Building from PKGBUILD

The repository includes a `PKGBUILD` for building on Arch Linux.

```bash
# Clone the repository
git clone https://github.com/brainless/dwata.git
cd dwata

# Build the package
makepkg -si

# This will:
# - Download dependencies
# - Build dwata-api
# - Install it to /usr/bin/dwata-api
```

## Option 3: Install from AUR (When Available)

Once published to AUR, you can install using an AUR helper:

```bash
# Using yay
yay -S dwata-api

# Using paru
paru -S dwata-api
```

## Post-Installation

After installation, dwata will:
- Store configuration in `~/.config/dwata/config.toml`
- Store database in `~/.local/share/dwata/db.sqlite`
- Use your system keyring for credential storage (requires `gnome-keyring` or similar)

Run the API server:
```bash
dwata-api
```

The server will start on `http://127.0.0.1:8080` by default.

## Dependencies

The static binary has no runtime dependencies. For the PKGBUILD build, you need:
- `rust` and `cargo` (build time)
- `git` (build time)
- `nodejs` and `npm` (build time)
- `gnome-keyring` or `libsecret` (runtime, for credential storage)
