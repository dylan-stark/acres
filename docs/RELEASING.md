# Releasing

A release is a version tag on `main`. Pushing the tag runs the CD workflow, which builds the `acres-cli` and `acres-tui` binaries and attaches them to a GitHub release.
The crates are prepared for crates.io but not published yet.

## Before the tag

Do these on a branch, in one pull request:

1. Bump `version` in each crate's `Cargo.toml` to the new version, and run `cargo build` to update `Cargo.lock`.
2. In `CHANGELOG.md`, rename `[Unreleased]` to `[X.Y.Z] - YYYY-MM-DD`, start a new empty `[Unreleased]` above it, and add a compare link for the new version at the bottom.
3. Run `make check`.
4. Check that each crate meant for crates.io would package, in dependency order:

   ```sh
   cargo publish --dry-run -p iiif
   cargo publish --dry-run -p charaoke
   cargo publish --dry-run -p acres
   ```

5. Merge the pull request.

## Tag

```sh
git switch main && git pull
git tag vX.Y.Z
git push origin vX.Y.Z
```

## After the tag

- The [CD workflow](../.github/workflows/cd.yml) builds a release binary for each target in its matrix. For each binary it uploads a `.tar.gz` and its `.sha256`, named like `acres-tui-vX.Y.Z-<os>-<arch>.tar.gz`.
- Check that the GitHub release lists every expected asset, and that a downloaded binary runs `--version`.
- Edit the release notes to summarize the changelog section.
