# gdenv

The best command-line tool to install and switch between multiple versions of Godot.

_Inspired by [xcodes](https://github.com/XcodesOrg/xcodes) and built with ❤️ in Rust._

## Installation

### Quick Install (Recommended)

```bash
# Unix/Linux/macOS
curl -fsSL https://gdenv.bytemeadow.com | sh

# Windows PowerShell
irm https://gdenv.bytemeadow.com | iex
```

### Cargo

```bash
# For the latest unstable version
cargo install --git https://github.com/bytemeadow/gdenv
# For the version released to crates.io
cargo install gdenv
```

### Manual Download

Download pre-built binaries from [GitHub Releases](https://github.com/bytemeadow/gdenv/releases)

---

<div align="left" valign="middle">
<a href="https://runblaze.dev">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://www.runblaze.dev/logo_dark.png">
   <img align="right" src="https://www.runblaze.dev/logo_light.png" height="102px"/>
 </picture>
</a>

<br style="display: none;"/>

_Special thanks to [Blaze](https://runblaze.dev) for their support of this project. They provide high-performance Linux (AMD64 & ARM64) and Apple Silicon macOS runners for GitHub Actions, greatly reducing our automated build times._

</div>

## Usage

Install a specific version of Godot using commands like:

```bash
gdenv install 4.4.1
gdenv install 4.5-beta1
gdenv install 3.6 --dotnet
gdenv install --latest
gdenv install --latest-prerelease
```

gdenv will download and install the version you asked for so that it's ready to use.

### `.godot-version`

We recommend creating a `.godot-version` file to explicitly declare the Godot version for your project:

```txt
4.4.1
```

Then run:
```bash
gdenv install  # Installs 4.2.1
gdenv use      # Switches to 4.2.1
```

### Commands

- `install <version>`: Download and install a specific version of Godot
- `installed`: List the versions of Godot that are installed
- `list`: List all versions of Godot that are available to install
- `use <version>`: Switch to a specific version of Godot
- `current`: Show the currently active version
- `uninstall <version>`: Uninstall a specific version of Godot
- `update`: Update the list of available versions of Godot
- `cache`: Manage download cache

## GitHub Action

`gdenv` can be used directly as a GitHub Action:

```yaml
- name: Setup Godot
  uses: bytemeadow/gdenv@v0.2.2
  with:
    version: 4.5.1
    gdenv-version: 0.2.2
    use-dotnet: false
    include-templates: false
    cache: true
```

### Action inputs

- `version` (required): Godot version to install and activate.
- `use-dotnet` (default: `false`): install/use .NET Godot builds.
- `include-templates` (default: `false`): install export templates for the selected version.
- `cache` (default: `true`): cache gdenv data and templates between runs.
- `gdenv-version` (default: `latest`): gdenv version to install.
- output `godot-bin`: absolute path to the resolved Godot executable.

The action adds `godot` to `PATH` and sets `GODOT`, `GODOT4`, and `GODOT4_BIN` to `godot`.

## License

gdenv is distributed under the terms of both the MIT license and the Apache License (Version 2.0).
See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT) for details. Opening a pull
request is assumed to signal agreement with these licensing terms.
