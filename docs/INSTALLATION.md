# Installation Guide

## System Requirements

- **Rust**: 1.70 or higher
- **Operating Systems**: Linux, macOS, Windows
- **Memory**: Minimum 512MB RAM (1GB recommended)
- **Disk Space**: 100MB for installation, additional space for cache

## Installation Methods

### From Crates.io (Recommended)

```bash
cargo install lspbridge
```

### From Source

To install the latest development version:

```bash
git clone https://github.com/Hydepwns/LSPbridge
cd LSPbridge
cargo install --path .
```

### With Specific Features

Install with specific feature flags:

```bash
# Minimal installation without Git integration
cargo install lspbridge --no-default-features --features cli

# With experimental features
cargo install lspbridge --features experimental

# With network support
cargo install lspbridge --features network
```

### Platform-Specific Instructions

#### Linux
```bash
# Debian/Ubuntu
sudo apt-get install build-essential pkg-config libssl-dev
cargo install lspbridge

# Arch Linux (AUR coming soon)
yay -S lspbridge
```

#### macOS
```bash
# Ensure Xcode tools are installed
xcode-select --install
cargo install lspbridge

# Homebrew (coming soon)
brew install lspbridge
```

#### Windows
```bash
# Install Visual Studio Build Tools first
cargo install lspbridge
```

## IDE Extension Installation

### Visual Studio Code

```bash
code --install-extension lsp-bridge
```

Or search for "LSP Bridge" in the Extensions panel.

### Neovim

Using [lazy.nvim](https://github.com/folke/lazy.nvim):

```lua
{
  "Hydepwns/lsp-bridge.nvim",
  dependencies = {
    "nvim-lua/plenary.nvim",
  },
  config = function()
    require("lsp-bridge").setup({
      -- Configuration options
      privacy_level = "default",
      auto_export = true,
    })
  end,
}
```

Using [packer.nvim](https://github.com/wbthomason/packer.nvim):

```lua
use {
  'Hydepwns/lsp-bridge.nvim',
  requires = { 'nvim-lua/plenary.nvim' },
  config = function()
    require('lsp-bridge').setup{}
  end
}
```

### Zed

Search for "LSP Bridge" in Zed's extension manager (coming soon).

## Post-Installation

```bash
# Verify installation
lsp-bridge --version

# Initialize configuration
lsp-bridge config init

# Test
echo '{"uri":"test.rs","diagnostics":[]}' | lsp-bridge export --format json
```

IDE extensions will automatically detect the binary if it's in your PATH.

## Updating LSP Bridge

### Update CLI Tool

```bash
# Update to latest version
cargo install lspbridge --force

# Update to specific version
cargo install lspbridge --version 0.3.0 --force
```

### Update IDE Extensions

- **VS Code**: Updates automatically or via Extensions panel
- **Neovim**: Update via your plugin manager
- **Zed**: Updates automatically

## Troubleshooting

### Common Issues

#### "command not found: lsp-bridge"

The binary is not in your PATH. Add Cargo's bin directory to your PATH:

```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.cargo/bin:$PATH"
```

#### Permission Denied

On Unix systems, ensure the binary has execute permissions:

```bash
chmod +x ~/.cargo/bin/lsp-bridge
```

#### Build Failures

Ensure you have the required dependencies:

```bash
# macOS
xcode-select --install

# Linux
sudo apt-get install build-essential pkg-config libssl-dev

# Windows
# Install Visual Studio Build Tools
```

### Getting Help

- Check [GitHub Issues](https://github.com/Hydepwns/LSPbridge/issues)
- Enable debug logging: `LSP_BRIDGE_LOG_LEVEL=debug lsp-bridge export`
- Report bugs with OS, Rust version, and error logs

## Uninstallation

### Remove CLI Tool

```bash
cargo uninstall lspbridge
```

### Remove Configuration

```bash
# Remove user configuration
rm ~/.config/lspbridge/lspbridge.toml

# Remove cache
rm -rf ~/.cache/lspbridge  # Linux/macOS
rm -rf ~/Library/Caches/lspbridge  # macOS alternative
rm -rf %LOCALAPPDATA%\lspbridge\cache  # Windows
```

### Remove IDE Extensions

- **VS Code**: Uninstall from Extensions panel
- **Neovim**: Remove from plugin configuration and run `:PackerClean` or equivalent
- **Zed**: Uninstall from Extensions panel