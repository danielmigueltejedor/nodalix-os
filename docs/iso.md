# ISO de Nodalix 0.2.0

Medio x86_64 creado con Archiso. Arranque BIOS y UEFI, sesión live Nodalix y
asistente Archinstall con el perfil Nodalix preseleccionado. La instalación
requiere Internet para obtener dependencias de Arch y los paquetes adecuados
al equipo. El usuario selecciona disco, idioma, cifrado y cuenta y confirma el
resumen antes de escribir en disco. No se selecciona ni borra un disco desde
los scripts de Nodalix.

El sistema instalado recibe paquetes Nodalix comprobados contra el manifiesto
SHA-256 incluido en la ISO, configuración de monitores automática, red, audio,
Bluetooth y pantalla de acceso. Los usuarios, claves, cuentas y preferencias
personales del equipo donde se compila no se copian.

El perfil conserva Linux de Arch y sus headers. En máquinas virtuales y CPU
anteriores a x86-64-v3 mantiene esa base compatible. En equipos físicos v3/v4
y Zen 4/5 intenta aplicar el perfil CachyOS, con kernel deckify para dispositivos
reconocidos. Una optimización fallida queda registrada en
`/var/log/nodalix-installer/hardware.json`; nunca se presenta como aplicada.
La selección gráfica predeterminada usa Mesa, con propuesta de NVIDIA open en
generaciones identificadas como Turing o posteriores. Puede revisarse en el
asistente. La compatibilidad de cada GPU, Wi-Fi y firmware requiere pruebas en
hardware real; la validación virtual no equivale a cobertura universal.

Secure Boot no viene firmado: debe desactivarse para arrancar esta ISO. El
usuario puede configurar sus propias claves después de instalar. Las opciones
de particionado, cifrado y recuperación proceden de Archinstall.

## Construcción

En Arch con archiso, a partir de paquetes cuyo manifiesto sea de esta versión:

```
sudo python3 tools/build-iso.py --assets /ruta/paquetes --work /ruta/nueva/construccion --output /ruta/iso
```

Cada construcción usa un directorio nuevo. El constructor no formatea discos;
solo crea el sistema de archivos de la imagen y su archivo ISO. Los repositorios
de Arch mantienen la validación de firmas. El repositorio local de paquetes
Nodalix se usa únicamente después de validar sus tamaños y SHA-256.

## Validación

La imagen no debe publicarse hasta comprobar arranque de la ISO y una
instalación en discos virtuales desechables. El informe de pruebas se añade
junto a la imagen publicada. Nunca se usan discos físicos del equipo de
construcción en estas pruebas.
