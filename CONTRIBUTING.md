# Contributing

Open a pull request against `main` with the reason for the change, its scope and the validation you ran. Link the issue when there is one. Include tests for behavior changes and record any checks you could not run.

Ask a maintainer who did not author the PR to approve it. Address feedback, resolve review conversations and confirm that CI passed on the final commit before merging.

Read the Nodalia [engineering](https://github.com/getnodalia/foundation/blob/main/standards/engineering.md), [CI](https://github.com/getnodalia/foundation/blob/main/standards/ci.md) and [release](https://github.com/getnodalia/foundation/blob/main/standards/release.md) standards.

See [AGENTS.md](AGENTS.md) for repository-specific development and testing instructions.

## Local checks

- `/usr/bin/python3 -m unittest discover -s tests -v`
- `bash tools/test-native-restart.sh`

Changes to the kernel, packages, signing, ISO, installer or updater also need the applicable [CI validation](.github/workflows/ci.yml) and release review. See the [package repository](docs/nodalix-repo.md) and [kernel](docs/nodalix-kernel.md) guides.
