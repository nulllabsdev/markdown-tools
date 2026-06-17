# Releasing markdown-tools

This document explains how to publish new versions of `markdown-tools` (both `gomd` and `rustmd`) to Homebrew and GitHub.

## Release Overview

The release process is automated through GitHub Actions:

1. **Push a version tag** (`v0.2.0`, `v0.3.0`, etc.) to the repository
2. **GitHub Actions workflow triggers:**
   - Builds native binaries for 4 platforms (macOS Intel/ARM, Linux x86-64/ARM64)
   - Creates archives and SHA-256 checksums
   - Publishes a GitHub Release with all artifacts
   - Updates the Homebrew tap formula with new version and checksums
3. **Users can install or upgrade** using `brew tap`, `brew install`, and `brew upgrade`

## Prerequisites

Ensure you have:

- Write access to `nulllabsdev/markdown-tools` repository
- The `HOMEBREW_TAP_TOKEN` secret configured in repository settings (created once, see setup docs)
- A clean working tree and up-to-date `main` branch

## Releasing a New Version

### Step 1: Update the Changelog

Edit `CHANGELOG.md` to document the release:

```markdown
## [0.2.0] - 2026-06-17

### Added
- New feature X
- New feature Y

### Fixed
- Bug fix A
- Bug fix B
```

Commit the changelog:

```bash
git add CHANGELOG.md
git commit -m "docs: prepare v0.2.0 changelog"
```

### Step 2: Update the Rust version

The release workflow validates that the git tag matches `rust/Cargo.toml`. Update the version:

Edit `rust/Cargo.toml`:

```toml
[package]
version = "0.2.0"
```

The Go binary gets its version from the git tag, so no change needed there.

### Step 3: Run the release script

The existing `scripts/release.sh` validates and prepares the release:

```bash
./scripts/release.sh v0.2.0
```

This script will:

- Validate the tag format
- Verify the changelog contains the release entry
- Update `rust/Cargo.toml` with the version (if not already done)
- Refresh `rust/Cargo.lock`
- Create a release commit (if needed)
- Create an annotated git tag

### Step 4: Push the commit and tag

Push to the upstream repository:

```bash
git push origin main --follow-tags
```

This pushes:
- The release commit to `main`
- The annotated `v0.2.0` tag (which triggers the workflow)

**Important:** Use `--follow-tags` to push annotated tags, or explicitly push the tag:

```bash
git push origin main
git push origin v0.2.0
```

### Step 5: Monitor the workflow

Open the Actions tab in GitHub:

https://github.com/nulllabsdev/markdown-tools/actions

Watch the "Release" workflow run. It should complete in 2-5 minutes with:

- ✅ 4 build jobs (Linux x86-64, Linux ARM64, macOS Intel, macOS ARM64)
- ✅ 1 publish job (creates GitHub Release and updates Homebrew tap)

All jobs should show green checkmarks (success).

### Step 6: Verify the release

Once the workflow completes:

1. **Check the GitHub Release:**
   - https://github.com/nulllabsdev/markdown-tools/releases/tag/v0.2.0
   - Should have 8 files (4 `.tar.gz` archives + 4 `.sha256` checksums)

2. **Check the Homebrew tap:**
   - https://github.com/nulllabsdev/homebrew-tap/commits/main
   - Should have a new commit `markdown-tools v0.2.0`
   - Formula at `Formula/markdown-tools.rb` should have the new version and checksums

3. **Test locally** (on another machine or in a container):
   ```bash
   brew tap nulllabsdev/tap
   brew install markdown-tools
   gomd -v    # should print: build v0.2.0
   rustmd -v  # should print: build v0.2.0
   ```

## For End Users: Installation and Upgrades

### First-time installation

```bash
brew tap nulllabsdev/tap
brew install markdown-tools
```

### Upgrading to a new version

```bash
brew update          # pulls the latest tap recipes
brew upgrade markdown-tools
```

### Checking installed version

```bash
gomd -v
rustmd -v
```

### Pinning a version (prevent automatic upgrades)

```bash
brew pin markdown-tools
```

### Unpinning and upgrading

```bash
brew unpin markdown-tools
brew upgrade markdown-tools
```

## Troubleshooting

### Workflow failed to build

Check the Actions log for the failing job. Common causes:

- **Version mismatch:** Tag is `v0.2.0-rc1` but `rust/Cargo.toml` says `0.1.0`
  - Fix: Update `Cargo.toml` before running `scripts/release.sh`

- **Runner unavailable:** A platform-specific runner is down
  - Wait a few minutes or check GitHub Actions runner status
  - If critical, remove that platform from the matrix temporarily

- **Cargo/Go build failure:** Dependency or compilation issue
  - Check the full build log
  - Fix locally with `make test` before retrying the release

### GitHub Release failed but builds succeeded

The publish job has issues (usually permissions). Check the log for:

- **Token error:** `HOMEBREW_TAP_TOKEN` is missing, expired, or lacks write access
  - Verify the secret is configured in Settings → Secrets and variables → Actions
  - Regenerate the token if expired

- **Git error:** `fatal: not a git repository`
  - This is a workflow bug; check that the publish job has a checkout step

### Homebrew tap didn't update

The release job succeeded but the tap formula is stale. This usually means:

- The token lacks write access to the tap repository
- The tap repository branch protection blocked the push
- The workflow is in a feature branch and not building the default branch

Fix by:

1. Verify `HOMEBREW_TAP_TOKEN` has `Contents: Read and write` on `nulllabsdev/homebrew-tap`
2. Check branch protection in homebrew-tap repository
3. Ensure future releases are tagged on the default branch

### Users can't find the new version with `brew update`

Run on the user's machine:

```bash
brew update --verbose
brew info nulllabsdev/tap/markdown-tools
```

If the tap is stale:

```bash
brew untap nulllabsdev/tap
brew tap nulllabsdev/tap
brew info nulllabsdev/tap/markdown-tools
```

## Release Checklist

Use this checklist for each release:

- [ ] Update and date the CHANGELOG.md release section
- [ ] Run `make test` locally to verify all tests pass
- [ ] Update `rust/Cargo.toml` version if not done already
- [ ] Run `./scripts/release.sh vX.Y.Z`
- [ ] Inspect the release commit and annotated tag with `git log --oneline -2` and `git tag -v vX.Y.Z`
- [ ] Run `git push origin main --follow-tags`
- [ ] Monitor the GitHub Actions run; all jobs should pass (green checkmarks)
- [ ] Verify the GitHub Release has all 8 files (4 archives + 4 checksums)
- [ ] Verify `homebrew-tap` has a new commit with the formula update
- [ ] Test on at least one machine: `brew tap nulllabsdev/tap && brew install markdown-tools`
- [ ] Verify both `gomd -v` and `rustmd -v` print the correct version

## For Maintainers: Token Rotation

The `HOMEBREW_TAP_TOKEN` in repository secrets should be rotated periodically:

1. Create a new fine-grained token in GitHub:
   - Settings → Developer settings → Personal access tokens → Fine-grained tokens
   - Scope: only `nulllabsdev/homebrew-tap`
   - Permission: **Contents — Read and write**
   - Expiration: 90 days

2. Update the secret:
   - Repository Settings → Secrets and variables → Actions
   - Click `HOMEBREW_TAP_TOKEN`
   - Update the value

3. Delete the old token in GitHub settings

## References

- [Homebrew Taps](https://docs.brew.sh/Taps)
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [GitHub Actions: Releasing projects in a repository](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
- [Release workflow definition](.github/workflows/release.yml)
- [Homebrew tap repository](https://github.com/nulllabsdev/homebrew-tap)
