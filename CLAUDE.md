# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

`gdenv` is a Rust CLI tool for installing and switching between multiple versions of Godot. It is inspired by `xcodes` for Xcode version management.

## Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build (with LTO, stripped, single codegen unit)
cargo test                     # Run all tests
cargo test <test_name>         # Run a specific test by name
cargo fmt                      # Format code
cargo fmt -- --check           # Check formatting without writing
cargo clippy -- -D warnings    # Lint (CI treats warnings as errors)
```

## Workspace structure

This is a Cargo workspace with two crates:

- **`gdenv/`** — binary crate: CLI parsing (`clap`), command dispatch, UI helpers
  - `src/cli.rs` — top-level `Cli` struct and `Commands`/`GodotCommands` enums
  - `src/commands/` — one file per subcommand (e.g. `godot/install.rs`, `godot/list.rs`, `sync.rs`, `run.rs`, `editor.rs`)
  - `src/ui.rs` — shared terminal formatting helpers

- **`gdenv-lib/`** — library crate: all core logic, independently testable
  - `config.rs` — `Config` struct; resolves data dir paths (`~/.local/share/gdenv` on Linux); `Config::setup()` is the main entry point
  - `godot_version.rs` — `GodotVersion` struct with regex-based version parsing, ordering, and `version_buffet()` for the list display
  - `project_specification.rs` — loads `gdenv.toml` or `.godot-version` by searching the current dir and all parents; `gdenv.toml` takes precedence
  - `installer.rs` — `ensure_installed`, `install_version_from_archive`, `set_active_version`, `list_installed`, `uninstall_version`; manages the `installations/` directory and symlinks
  - `github.rs` — `GitHubClient` implementing `DownloadClient`; fetches Godot releases from GitHub API (`godotengine/godot-builds`); caches results in `cache/releases_cache.json`
  - `addons.rs` — `sync_addons` syncs Git and local addons into the Godot project's `addons/` directory
  - `git.rs` — `GitClient` trait and implementation for cloning/fetching addon repos into `cache/git_cache/`
  - `file_sync.rs` — recursive directory sync with include/exclude filtering
  - `gdextension_config.rs` — generates `.gdextension` files for Rust GDExtension crates
  - `cargo.rs` — reads `cargo metadata` to find the Cargo target directory for GDExtension builds
  - `migrate.rs` — handles data directory format migrations
  - `download_client.rs` — `DownloadClient` trait abstracting HTTP downloads (used for testing via `MockDownloadClient`)
  - `logging.rs` — initializes `tracing` with `tracing-indicatif` progress bars

## Key data flows

**Install flow:** CLI → `installer::ensure_installed` → `DownloadClient::godot_releases` (cache or GitHub API) → `download_asset` → `install_version_from_archive` (unzips to `installations/godot-<version>/`)

**Use/activate flow:** `installer::set_active_version` creates two symlinks: `current → installations/godot-<version>` and `bin/godot → <executable inside installation>`

**Project resolution:** Every command that needs a project calls `project_specification::load_godot_project_spec`, which walks up from `--project` (default: CWD) looking for `gdenv.toml` then `.godot-version`.

**Addon sync:** `addons::sync_addons` reads `ProjectSpecification::addons`, clones/fetches each Git repo into `git_cache/`, then uses `file_sync::sync_recursive` to copy files into the Godot project's `addons/<name>/` directory.

## Testing approach

Integration tests use `tempfile` to create isolated temporary directories and pass them as the `--datadir`. The `MockDownloadClient` in `gdenv-lib/src/test_helpers/` provides canned GitHub API responses backed by actual Godot zip fixtures, so tests exercise the full install pipeline without network access.

Run a single test:
```bash
cargo test test_installation_lifecycle
```
