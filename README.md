# gdenv - Godot Environment Manager

A beautiful terminal tool for managing Godot installations, inspired by [xcodes](https://github.com/XcodesOrg/xcodes).

## Features

✨ **Simple & Beautiful** - Clean CLI interface with colored output and progress bars
🚀 **Fast Downloads** - Streaming downloads with progress indicators
🔄 **Version Switching** - Seamless switching between Godot versions via symlinks
📦 **Smart Management** - Standard version by default, .NET available with --dotnet flag
🌍 **Cross-Platform** - Works on Windows, macOS, and Linux
⚡ **Zero Config** - Works out of the box with sensible defaults

## Installation

```bash
cargo install --path .
```

## Usage

### Install a Godot Version

```bash
# Install latest stable
gdenv install 4.2.1

# Install with .NET support
gdenv install 4.2.1 --dotnet

# Install a beta/rc version
gdenv install 4.3.0-beta2

# Force reinstall
gdenv install 4.2.1 --force

# Install from .godot-version file
gdenv install
```

### List Versions

```bash
# Show available versions from GitHub
gdenv list

# Include prereleases (beta, rc, etc.)
gdenv list --include-prereleases

# Show installed versions
gdenv installed

# Show installed versions with paths
gdenv installed --path
```

### Switch Between Versions

```bash
# Switch to a specific version
gdenv use 4.2.1

# Switch to .NET version
gdenv use 4.1.0 --dotnet

# Switch to version from .godot-version file
gdenv use
```

### Check Current Version

```bash
# Show active version
gdenv current

# Show path to active Godot
gdenv current --path
```

### Uninstall Versions

```bash
# Uninstall with confirmation
gdenv uninstall 4.1.0

# Skip confirmation
gdenv uninstall 4.1.0 --yes
```

### Update Available Versions

```bash
# Refresh available versions from GitHub
gdenv update
```

### Cache Management

```bash
# Show cache info
gdenv cache
gdenv cache info

# Clear download cache
gdenv cache clear
```

### Version Files

Create a `.godot-version` file in your project root:

```bash
echo "4.2.1" > .godot-version

# Now these commands will use that version:
gdenv install  # Installs 4.2.1
gdenv use      # Switches to 4.2.1
```

## How It Works

- **Installations**: Stored in `~/.local/share/gdenv/installations/` (Linux/macOS) or `%APPDATA%/gdenv/installations/` (Windows)
- **Active Version**: Managed via symlink at `~/.local/share/gdenv/current/`
- **Downloads**: Cached in `~/.local/share/gdenv/cache/` for faster reinstalls
- **Sources**: Fetches releases from [godotengine/godot-builds](https://github.com/godotengine/godot-builds)

## Examples

```bash
# Fresh setup workflow
gdenv install 4.2.1          # Install latest stable
gdenv current                 # Verify it's active
gdenv installed               # See installed versions

# Multi-version workflow
gdenv install 4.3.0-beta2    # Install beta for testing
gdenv install 4.1.0 --dotnet # Install older version with .NET for compatibility
gdenv installed               # See all versions with active indicator (★)
gdenv use 4.3.0-beta2        # Switch to beta
gdenv use 4.2.1              # Switch back to stable

# Browse available versions
gdenv list                    # See all available versions
gdenv list --include-prereleases  # Include beta/rc versions

# Cleanup
gdenv uninstall 4.1.0        # Remove old version
gdenv installed               # Verify removal
```

## Architecture

gdenv takes inspiration from xcodes' excellent design and adapts it for Godot:

- **Simple Commands**: Clean `install`, `list`, `installed`, `use`, `uninstall` API
- **Godot-Specific**: Proper version parsing, .NET opt-in support, GitHub integration  
- **Modern Rust**: Async downloads, robust error handling, beautiful terminal UI

## Contributing

Built with ❤️ using:
- [clap](https://github.com/clap-rs/clap) - CLI framework
- [tokio](https://tokio.rs/) - Async runtime
- [reqwest](https://github.com/seanmonstar/reqwest) - HTTP client
- [indicatif](https://github.com/console-rs/indicatif) - Progress bars
- [colored](https://github.com/mackwic/colored) - Terminal colors

## License

MIT
