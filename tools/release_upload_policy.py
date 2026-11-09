#!/usr/bin/env python3
"""Decide whether assets may be uploaded to a GitHub Release.

Published stable releases are immutable, including the zero-asset case.
Drafts and prereleases may receive uploads (and clobber).
"""

from __future__ import annotations

import argparse
import json
import sys


def allow_asset_upload(*, is_draft: bool, is_prerelease: bool) -> bool:
    return bool(is_draft) or bool(is_prerelease)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--release-json",
        required=True,
        help="JSON from `gh release view --json isDraft,isPrerelease,...`",
    )
    parser.add_argument(
        "--tag",
        default="",
        help="Release tag used in the refusal message",
    )
    args = parser.parse_args(argv)
    payload = json.loads(args.release_json)
    is_draft = bool(payload.get("isDraft"))
    is_prerelease = bool(payload.get("isPrerelease"))
    if allow_asset_upload(is_draft=is_draft, is_prerelease=is_prerelease):
        print("allow")
        return 0
    tag = args.tag or payload.get("tagName") or payload.get("name") or "release"
    print(
        f"Refusing to upload assets to published stable release {tag}; "
        "return it to draft first.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
