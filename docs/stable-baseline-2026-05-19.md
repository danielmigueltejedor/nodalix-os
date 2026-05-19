# Estado base estable Nodalix Desktop — 2026-05-19

## Resumen

- Zen Browser como navegador principal.
- Firefox como navegador de respaldo.
- Bitwarden como gestor de contraseñas.
- LibreOffice pulido con script Nodalix.
- Waybar con islas y popups Nodalix estabilizados.
- firewalld como firewall único.
- Snapper + cachyos-snapper-support + limine-snapper-sync para snapshots.
- Bruno como cliente API.
- Satty como herramienta de capturas.
- LocalSend permitido por firewall en 53317/tcp y 53317/udp.

## Verificación

```text
Repo: limpio y sincronizado con origin/main
firewalld: active
BROWSER: zen-browser
HTTP/HTML default: zen.desktop
Orphans: none
```
