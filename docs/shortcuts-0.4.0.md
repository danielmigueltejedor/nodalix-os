# Atajos predeterminados de Nodalix OS 0.4.0

| Atajo | Acción |
| --- | --- |
| Super sola | Abrir la vista de GNOME |
| Super+C | Copiar |
| Super+V | Pegar |
| Super+Enter | Abrir la terminal predeterminada |
| Super+B | Abrir el navegador predeterminado |
| Super+M | Abrir el correo predeterminado |
| Super+Q | Cerrar la ventana enfocada completa |
| Super+Shift+3 | Guardar una captura de toda la pantalla |
| Super+Shift+4 | Abrir el selector nativo para capturar una zona |

La ventana bajo el ratón recibe el foco sin clicar. No se eleva automáticamente.
Las aplicaciones se abren mediante los valores predeterminados del usuario.

Los atajos se registran con GNOME: cierre y capturas usan sus acciones nativas;
los lanzadores y copiar/pegar aparecen en los atajos personalizados de Ajustes.
No se instala una extensión para capturar el teclado. Los valores personalizados
existentes se conservan y no se ocupa una combinación ya asignada por el usuario.

GNOME no dispone de una acción global de copiar/pegar. El pequeño servicio
`nodalix-clipboard` emite únicamente Ctrl+Insert y Shift+Insert. Espera a que se
suelten realmente las teclas modificadoras y comprueba que quien lo solicita es
el propietario de la sesión local activa y desbloqueada. Consulta el estado de
los modificadores; no lee eventos de texto, contenido del portapapeles ni ventanas.
No admite secuencias arbitrarias. Las aplicaciones deben admitir esas combinaciones.
Ghostty recibe una configuración de portapapeles explícita para Insert, evitando
la selección primaria y manteniendo Ctrl+C para interrumpir el proceso.

La migración libera primero las combinaciones de la antigua extensión Nodalix
Shortcuts y después registra los atajos nativos. Retira únicamente los atajos de
fábrica Super+Q y Super+Shift+3/4 del dock que competían con cierre y capturas.
Archiva la configuración antigua exacta de keyd bajo
`/var/lib/nodalix-updater/migrations/gnome-0.4.0/`; las configuraciones personalizadas
se conservan. El servicio keyd se detiene si ya no quedan configuraciones.

Las extensiones sustituidas y las copias locales que ocultaban versiones del
sistema se archivan bajo
`~/.local/state/nodalix/migrations/gnome-0.4.0-native-extensions/`.
Tiling Assistant se conserva para colocar ventanas manualmente; el mosaico automático es opcional y respeta las ventanas colocadas con Tiling Assistant. Solo queda un proveedor de paneles secundarios. Las preferencias
y extensiones ajenas se conservan. Se necesita volver a iniciar sesión para cargar
los módulos nuevos de GNOME Shell; la actualización no cierra aplicaciones.

Las pruebas usan un compositor GNOME 51 real con bus, usuario temporal y monitores
virtuales propios. Verifican GTK, Qt, Ghostty, Ctrl+C, capturas, foco con el ratón,
cierre de ventanas completas y despliegue del dock en ambos monitores.
