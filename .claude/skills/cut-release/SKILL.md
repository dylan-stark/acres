---
name: cut-release
description: Cut an acres release: bump the versions, close the changelog section, dry-run packaging, open the release PR, then tag main once it merges. Use when asked to prepare, cut or tag a release.
---

# Cut a release

`docs/RELEASING.md` is the reference. This skill is the same steps in the order an agent runs them, with the checks between them.

## 1. Prepare the branch

1. Start a branch from an up-to-date `main`.
2. Pick the version `X.Y.Z` (ask if it was not given). Read `CHANGELOG.md` under `[Unreleased]`: any `Removed` entry or breaking `Changed` entry means a minor bump while the version is `0.x`.
3. Set `version = "X.Y.Z"` in every crate's `Cargo.toml`, and the `version` on every workspace path dependency to match:

   ```sh
   grep -n '^version\|path = "\.\./' */Cargo.toml
   ```

4. Run `cargo build --workspace` so `Cargo.lock` picks up the new versions.

## 2. Close the changelog section

1. Rename `## [Unreleased]` to `## [X.Y.Z] - YYYY-MM-DD` (today's date) and put a new, empty `## [Unreleased]` above it.
2. Open the section with a few lines on what the release is about, for someone deciding whether to upgrade. Keep every entry below it.
3. At the bottom, point `[Unreleased]` at `compare/vX.Y.Z...HEAD` and add `[X.Y.Z]: .../compare/v<previous>...vX.Y.Z`.

## 3. Check

1. `make check` must pass.
2. Dry-run packaging of the crates meant for crates.io, all in one command so the unpublished local dependencies resolve:

   ```sh
   cargo publish --dry-run -p iiif -p charaoke -p acres
   ```

   Crates with `publish = false` (`acres-report`, `image-to-ascii-builder`) are skipped, and so are the apps that depend on them. Each packaged crate must list `description`, `license`, `repository` and `readme`.
3. `.github/workflows/cd.yml` builds the targets listed in `docs/RELEASING.md`. If the targets changed, update both.

## 4. Open the release PR

Commit with a message like `Prepared release X.Y.Z.`, push, and open a PR against `main` titled `Release X.Y.Z`. Do not merge it yourself.

## 5. Tag, after the PR merges

Tagging publishes a GitHub release, so confirm with the person first. Then:

```sh
git switch main && git pull
git log -1 --format=%s   # the release PR's commit
git tag vX.Y.Z
git push origin vX.Y.Z
```

If the merge landed on a later date than the changelog says, fix the date in a small PR first.

## 6. Check the release

1. Watch the CD run for the tag. Every matrix job must pass.
2. The GitHub release must list a `.tar.gz` and a `.sha256` for `acres-cli` and `acres-tui` on each target, named `<bin>-vX.Y.Z-<os>-<arch>.tar.gz`.
3. Edit the release notes to the summary at the top of the changelog section, with a link to the section.
4. Report the release link and anything that failed.
