# Atajos predeterminados de Nodalix OS 0.4.0

| Atajo | Acción |
| --- | --- |
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

La extensión Nodalix Shortcuts adapta copiar y pegar a las terminales. Ghostty
usa sus acciones reales de portapapeles para evitar enviar Ctrl+C al proceso
o introducir secuencias de teclado cuando no hay selección. Ctrl+C conserva
su función de interrupción. Las demás terminales usan Ctrl+Shift+C/V.
El portapapeles se usa a través de la aplicación; no se guarda un historial.

Los lanzadores, el foco y las capturas se configuran solo si el usuario no ha
escrito ya una preferencia. Los atajos de copiar y pegar tienen sus propias
claves en el esquema `com.nodalix.shell.shortcuts`; se pueden cambiar o
inhabilitar con la extensión. La página unificada de atajos sigue pendiente.

La migración elimina únicamente las asignaciones de fábrica Super+Q y
Super+Shift+3/4 del dock que competían con el cierre y las capturas. Conserva asignaciones personalizadas.
También archiva y retira la configuración antigua exacta de Nodalix en keyd
que convertía Super+Q en Ctrl+W y Super+C/V en Ctrl+C/V. No retira configuraciones
personalizadas ni enlaces simbólicos. Los archivos archivados quedan bajo
`/var/lib/nodalix-updater/migrations/gnome-0.4.0/`.

La actualización instala estos cambios; las pruebas de preparación no los
instalan ni modifican la sesión real del usuario. Las pruebas de GNOME 51
usan otra sesión Wayland, dos monitores virtuales y un directorio de usuario
temporal. Verifican GTK, Qt, Ghostty, Ctrl+C, capturas, foco con el ratón y cierre.
