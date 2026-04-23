# systemd-tui

`systemd-tui` is a terminal user interface for monitoring and managing `systemd` units over D-Bus.

It is built in Rust and provides a keyboard-driven workflow for listing units, filtering by name/state,
and running actions like start, stop, restart, and reload.

## Install

### Linux

#### Binary

Download from [Releases](https://github.com/ptrvsrg/systemd-tui/releases) page:

```bash
curl -LO "https://github.com/ptrvsrg/systemd-tui/releases/download/v<VERSION>/systemd-tui_<VERSION>_linux_<ARCH>.tar.gz"
```

#### Linux packages

Releases also publish native packages:

- `.deb` (Debian/Ubuntu)
- `.rpm` (RHEL/Fedora/openSUSE)
- `.apk` (Alpine)

After downloading the appropriate package from the [Releases](https://github.com/ptrvsrg/systemd-tui/releases) page, install it with your system package manager:

```bash
# Debian / Ubuntu
sudo apt install ./systemd-tui_<VERSION>_<ARCH>.deb
```

```bash
# RHEL / Fedora / openSUSE
sudo rpm -i ./systemd-tui-<VERSION>-1.<ARCH>.rpm
```

```bash
# Alpine
sudo apk add --allow-untrusted ./systemd-tui-<VERSION>-r1.<ARCH>.apk
```

#### Snap

After downloading the appropriate package from the [Releases](https://github.com/ptrvsrg/systemd-tui/releases) page, install it with `snap`:

```bash
# x86_64
sudo snap install ./systemd-tui_<VERSION>_amd64.snap --dangerous --classic
```

```bash
# ARM64
sudo snap install ./systemd-tui_<VERSION>_arm64.snap --dangerous --classic
```

### macOS

#### Binary

Download from [Releases](https://github.com/ptrvsrg/systemd-tui/releases) page:

```bash
curl -LO "https://github.com/ptrvsrg/systemd-tui/releases/download/v<VERSION>/systemd-tui_<VERSION>_darwin_<ARCH>.tar.gz"
```

#### Homebrew Cask

Install via Homebrew tap maintained in this repository:

```bash
brew install --cask ptrvsrg/systemd-tui/systemd-tui
```

### Windows

#### Binary

Download from the [Releases](https://github.com/ptrvsrg/systemd-tui/releases) page:

```powershell
curl.exe -LO "https://github.com/ptrvsrg/systemd-tui/releases/download/v$Version/systemd-tui_<VERSION>_windows_amd64.zip"
Expand-Archive -Path "systemd-tui_<VERSION>_windows_amd64.zip" -DestinationPath .
.\systemd-tui.exe
```

### Docker

Container images are published to [GHCR](https://github.com/ptrvsrg/systemd-tui/pkgs/container/systemd-tui):

```bash
# Specific version
docker pull ghcr.io/ptrvsrg/systemd-tui:<VERSION>
```

```bash
# Latest version
docker pull ghcr.io/ptrvsrg/systemd-tui:latest
```

### Build from source

#### Requirements

- Rust toolchain (stable)
- `cargo`

#### Build

```bash
git clone https://github.com/ptrvsrg/systemd-tui.git
cd systemd-tui
cargo build --release
```

The binary will be available at:

- `target/release/systemd-tui` (Linux/macOS)
- `target/release/systemd-tui.exe` (Windows)
