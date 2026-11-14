# Branch validation and publication

- `quality` runs for every pull request as well as pushes to `main`.
- `CI` builds and verifies the kernel, builds the GNOME package set and checks migration, clean installation, ISO and signed-repository behavior. Check the related jobs before merging changes to those areas.
- `Publish pacman repository` is dispatched on `main` with a CI run ID. Use an eligible candidate whose validation and signing checks passed. See [the pacman repository guide](nodalix-repo.md).
- `Release Nodalix OS` runs on release tags and can be dispatched for an existing tag. Verify the version, release notes, checksums and signed assets. Review the [ISO guide](iso.md) before publishing.
