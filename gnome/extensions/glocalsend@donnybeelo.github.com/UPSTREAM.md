# GLocalSend preserved Nodalix snapshot

Upstream: https://github.com/donnybeelo/gnome-extensions-glocalsend, installed version 18.
Imported from the working installation on 2026-10-07, preserving Spanish strings,
fingerprint favorites, favorites-first sorting, GNOME 51 orientations, configurable
start-on-login and auto-disable behavior. GSettings schema and UUID are unchanged.
No certificate, private key or user preferences are stored here.

Nodalix changes: cancel the login timer on disable; use actor destruction instead
of private Quick Settings removal; expose discovery state on D-Bus for Nautilus.
File upload from Nautilus runs in a detached worker, outside GNOME Shell.
The upstream receive and Quick Settings transfer engine still runs inside Shell;
moving that engine into a standalone service is a remaining migration step.

This snapshot is included in the GNOME installation at the user's explicit
request. The upstream snapshot has no license file; no license is invented for
those third-party files. Other bundled extensions retain their license files.
