# Release notes

Every Nodalix version has its notes here, written by hand: `docs/release-notes/vX.Y.Z.md`.

## How a release happens

- Bump `VERSION` and add `docs/release-notes/vX.Y.Z.md` in the same pull request. The **Release
  notes are written** check (`scripts/check-release-notes.sh`, which runs
  `tools/release_notes.py validate`) fails the pull request if the notes are missing or
  incomplete.
- Merging it releases the version: **Release from main** (`release-auto.yml`) tags the merge with
  the bare version (`0.7.0`, the form installed updaters read) and calls `release.yml`, which
  builds the packages and, for a stable version, the ISO, validates the upgrade, and publishes the
  release with these notes followed by the list of changes since the previous tag. Nobody pushes
  a tag by hand.
- A version that fails stays unpublished. Fix it on `main` and run `release.yml` by hand with the
  tag, or bump to a new version.

## Rules (`tools/release_notes.py`)

- User-facing highlights as Markdown bullets, at least a few lines.
- No placeholders (TODO, TBD…), and no "What's changed" or "Full changelog": the release workflow
  adds those.
