#!/usr/bin/env python3
"""Seed gdenv release cache for a specific Godot version.

This avoids unauthenticated GitHub API pagination requests from gdenv 0.2.1
on CI runners by pre-writing releases_cache.json with a single release entry.
"""

from __future__ import annotations

import json
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

VERSION_RE = re.compile(
    r"^v?(\d+)(?:\.(\d+))?(?:\.(\d+))?(?:\.(\d+))?(?:-([a-zA-Z]+)(\d+)?)?(.*?)$",
)


def parse_version(version_str: str) -> dict:
    match = VERSION_RE.match(version_str)
    if not match:
        raise ValueError(f"Invalid Godot version format: {version_str}")

    major_s, minor_s, patch_s, sub_patch_s, rel_tag, tag_version_s, extra = match.groups()

    major = int(major_s)
    minor_raw = int(minor_s) if minor_s else None
    patch_raw = int(patch_s) if patch_s else None
    sub_patch_raw = int(sub_patch_s) if sub_patch_s else None

    sub_patch = sub_patch_raw if sub_patch_raw and sub_patch_raw > 0 else None
    patch = patch_raw if patch_raw is not None and (sub_patch is not None or patch_raw > 0) else None
    minor = minor_raw if minor_raw is not None and (patch is not None or minor_raw > 0) else None

    release_tag = rel_tag or "stable"
    tag_version = int(tag_version_s) if tag_version_s else None
    extra = extra if extra else None

    return {
        "major": major,
        "minor": minor,
        "patch": patch,
        "sub_patch": sub_patch,
        "release_tag": release_tag,
        "tag_version": tag_version,
        "extra": extra,
        "is_dotnet": False,
    }


def as_godot_version_str(version: dict) -> str:
    out = f"{version['major']}.{version['minor'] if version['minor'] is not None else 0}"
    if version["patch"] is not None:
        out += f".{version['patch']}"
        if version["sub_patch"] is not None:
            out += f".{version['sub_patch']}"

    out += f"-{version['release_tag']}"
    if version["tag_version"] is not None:
        out += str(version["tag_version"])
    if version["extra"] is not None:
        out += version["extra"]
    return out


def fetch_release(tag: str) -> dict:
    url = f"https://api.github.com/repos/godotengine/godot-builds/releases/tags/{tag}"
    request = urllib.request.Request(url)
    request.add_header("Accept", "application/vnd.github+json")
    request.add_header("X-GitHub-Api-Version", "2022-11-28")

    token = os.environ.get("GITHUB_TOKEN")
    if token:
        request.add_header("Authorization", f"Bearer {token}")

    with urllib.request.urlopen(request) as response:
        return json.load(response)


def main() -> int:
    if len(sys.argv) != 3:
        print("Usage: seed-release-cache.py <version> <cache_file>", file=sys.stderr)
        return 2

    version_input = sys.argv[1]
    cache_file = Path(sys.argv[2]).expanduser()

    version = parse_version(version_input)
    release_tag = as_godot_version_str(version)

    try:
        release = fetch_release(release_tag)
    except urllib.error.HTTPError as exc:
        print(f"Failed to fetch release '{release_tag}': HTTP {exc.code}", file=sys.stderr)
        return 1
    except urllib.error.URLError as exc:
        print(f"Failed to fetch release '{release_tag}': {exc.reason}", file=sys.stderr)
        return 1

    cache_payload = [
        {
            "version": version,
            "assets": [
                {
                    "name": asset["name"],
                    "browser_download_url": asset["browser_download_url"],
                    "size": asset.get("size", 0),
                }
                for asset in release.get("assets", [])
            ],
        },
    ]

    cache_file.parent.mkdir(parents=True, exist_ok=True)
    cache_file.write_text(json.dumps(cache_payload, indent=2), encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
