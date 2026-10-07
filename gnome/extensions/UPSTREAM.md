# GNOME 51 desktop profile

Snapshots from the working installation on 2026-10-08; metadata versions and
upstream URLs are retained in each directory. No user identities or keys are
included. Runtime schemas are compiled during packaging. Third-party copyright
headers and supplied license files are retained.

| Extension | Version | Upstream |
| --- | --- | --- |
| Dash to Dock | 109 | https://github.com/micheleg/dash-to-dock |
| Blur my Shell | 74 | https://github.com/aunetx/blur-my-shell |
| Tiling Assistant | 55 | https://github.com/ubuntu/Tiling-Assistant |
| Rounded Window Corners Native | 51.0 | https://github.com/ztrahmet/rounded-window-corners |
| GLocalSend | 18 + Nodalix changes | https://github.com/donnybeelo/gnome-extensions-glocalsend |
| Nodalix Connect | 2 | This repository |

Hanabi is built separately from the pinned revision in the video package.
`gnome/session/extensions.json` is the installation and migration profile.
Desktop appearance defaults omit this computer's physical monitor identifiers.
Existing preferences are never replaced by profile defaults.
