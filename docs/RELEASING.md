# Releasing

A release is a version tag on `main`. Pushing the tag runs the CD workflow, which builds the `acres-cli` and `acres-tui` binaries and attaches them to a GitHub release.
The crates are prepared for crates.io but not published yet. `acres-report` and `image-to-ascii-builder` set `publish = false`, so the apps that depend on `acres-report` can't be published as they are. The name `iiif` is already taken on crates.io, so that crate needs a new package name before it is published.

The `cut-release` Claude Code skill in `.claude/skills/` runs these steps in order.

## Before the tag

Do these on a branch, in one pull request:

1. Bump `version` in each crate's `Cargo.toml` to the new version, along with the `version` on each workspace path dependency, and run `cargo build` to update `Cargo.lock`.
2. In `CHANGELOG.md`, rename `[Unreleased]` to `[X.Y.Z] - YYYY-MM-DD`, start a new empty `[Unreleased]` above it, open the section with a short summary of the release, and update the compare links at the bottom.
3. Run `make check`.
4. Check that each crate meant for crates.io would package. Name them in one command, so cargo resolves the local dependencies that aren't on crates.io yet:

   ```sh
   cargo publish --dry-run -p iiif -p charaoke -p acres
   ```

5. Merge the pull request.

## Tag

```sh
git switch main && git pull
git tag vX.Y.Z
git push origin vX.Y.Z
```

## After the tag

- The [CD workflow](../.github/workflows/cd.yml) builds a release binary for each target in its matrix: Linux x86-64 and arm64, macOS arm64 and Windows x86-64, each on a native runner. The workspace's release profile strips debug symbols. For each binary it uploads a `.tar.gz` and its `.sha256`, named like `acres-tui-vX.Y.Z-<os>-<arch>.tar.gz`.
- Check that the GitHub release lists every expected asset, and that a downloaded binary runs `--version`.
- Edit the release notes to summarize the changelog section.
