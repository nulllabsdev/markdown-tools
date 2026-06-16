#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: scripts/release.sh vX.Y.Z[-prerelease]" >&2
  exit 1
fi

version="$1"
today=$(date +%F)

if ! printf '%s\n' "$version" | grep -Eq '^v[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.]+)?$'; then
  echo "invalid version: $version" >&2
  exit 1
fi

if ! git diff --quiet --ignore-submodules HEAD --; then
  echo "worktree is not clean" >&2
  exit 1
fi

if ! git diff --cached --quiet --ignore-submodules --; then
  echo "index is not clean" >&2
  exit 1
fi

plain_version=${version#v}
repo_root=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
changelog="$repo_root/CHANGELOG.md"
cargo_toml="$repo_root/rust/Cargo.toml"
cargo_lock="$repo_root/rust/Cargo.lock"

if git rev-parse "$version" >/dev/null 2>&1; then
  echo "tag already exists: $version" >&2
  exit 1
fi

if ! grep -Fq "## [$plain_version]" "$changelog"; then
  echo "CHANGELOG.md is missing entry for $plain_version" >&2
  exit 1
fi

if ! grep -Fq "## [$plain_version] - $today" "$changelog"; then
  echo "CHANGELOG.md entry for $plain_version must include today's date ($today)" >&2
  exit 1
fi

perl -0pi -e 's/version = "[^"]+"/version = "'"$plain_version"'"/' "$cargo_toml"

(
  cd "$repo_root/rust"
  cargo check >/dev/null
)

git add "$cargo_toml" "$cargo_lock"
if ! git diff --cached --quiet --ignore-submodules --; then
  git commit -m "chore(release): $version"
fi
git tag -a "$version" -m "Release $version"

echo "tagged $version"
