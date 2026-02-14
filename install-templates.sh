#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" = "" ]; then
  echo "Usage: install-templates.sh <version>" >&2
  exit 1
fi

version_input="$1"

if ! command -v godot >/dev/null 2>&1; then
  echo "godot executable was not found in PATH" >&2
  exit 1
fi

normalize_release_tag() {
  local version="$1"
  if [[ "$version" == *-* ]]; then
    printf "%s" "$version"
  else
    printf "%s-stable" "$version"
  fi
}

runtime_version="$(godot --version | awk '{print $1}')"

template_folder="$(printf '%s' "$runtime_version" | sed -E 's/^([0-9]+\.[0-9]+(\.[0-9]+)?\.[A-Za-z0-9]+).*/\1/')"
release_tag="$(printf '%s' "$template_folder" | sed -E 's/^([0-9]+\.[0-9]+(\.[0-9]+)?)\.([A-Za-z]+)([0-9]*)$/\1-\3\4/')"

if [ -z "$release_tag" ] || [ "$release_tag" = "$template_folder" ]; then
  release_tag="$(normalize_release_tag "$version_input")"
fi

if [ -z "$template_folder" ] || [ "$template_folder" = "$runtime_version" ]; then
  template_folder="${release_tag/-/.}"
fi

if [ "$(uname -s)" = "Darwin" ]; then
  templates_root="$HOME/Library/Application Support/Godot/export_templates"
else
  templates_root="$HOME/.local/share/godot/export_templates"
fi

destination="$templates_root/$template_folder"
if [ -f "$destination/version.txt" ]; then
  echo "Export templates already installed at $destination"
  exit 0
fi

mkdir -p "$destination"

download_url="https://github.com/godotengine/godot-builds/releases/download/${release_tag}/Godot_v${release_tag}_export_templates.tpz"
tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

archive_path="$tmp_dir/export_templates.tpz"
extract_dir="$tmp_dir/extracted"
mkdir -p "$extract_dir"

echo "Downloading export templates from ${download_url}"
curl -fL --retry 3 --retry-delay 2 "$download_url" -o "$archive_path"

if command -v unzip >/dev/null 2>&1; then
  unzip -q "$archive_path" -d "$extract_dir"
else
  python3 - "$archive_path" "$extract_dir" <<'PY'
import sys
import zipfile

archive_path = sys.argv[1]
extract_dir = sys.argv[2]

with zipfile.ZipFile(archive_path, "r") as archive:
    archive.extractall(extract_dir)
PY
fi

source_dir="$extract_dir"
if [ -d "$extract_dir/templates" ]; then
  source_dir="$extract_dir/templates"
fi

cp -R "$source_dir"/. "$destination"/
find "$destination" -maxdepth 1 -type f -name 'linux_*' -exec chmod +x {} \; || true

echo "Installed export templates to $destination"
