# Nodalix OS Privacy Model

Nodalix OS should be private and secure by default.

## Core ideas

- Microphone usage should be visible in Waybar
- Camera usage should be visible in Waybar
- Screen sharing and recording should be visible in Waybar
- AI listening or screen access must always be visible
- Screen access by AI should be ask-by-default
- Sudo and destructive shell commands should require confirmation

## Default policy

- Firewall enabled by default in future releases
- SSH disabled unless explicitly enabled
- Secrets stored through keyring/libsecret where possible
- No API keys in plain configuration files
- Portals configured with conservative defaults
