# Base GNOME de Nodalix

El escritorio oficial de desarrollo utiliza GNOME Shell 51, Mutter, GDM y Wayland.
La sesión Nodalix ejecuta `gnome-session --session=gnome`. GNOME gestiona ventanas,
Overview, espacios de trabajo, notificaciones, fondos y ajustes rápidos.

## Paquetes y actualización

- `nodalix-integrations`: servicios independientes del compositor: iCloud Drive,
  sincronización de calendario, organización de carpetas y utilidades de sistema.
- `nodalix-gnome`: sesión, portales GNOME/GTK, integración de Nautilus, controles
  simbólicos Adwaita y Nodalix Connect.
- `nodalix-control-center`: Ajustes GNOME con «Sistema → Actualizaciones» y
  «Apariencia → Fondos animados», y marca Nodalix en «Acerca de».
- `nodalix-settings`: servicio de actualizaciones independiente de Quickshell,
  selector de fondos y logo de GDM.
- `nodalix-video-wallpapers`: reproductor adaptado a GNOME 51 y los 16 vídeos
  de la colección actual.
La versión 0.3.0 reemplaza `nodalix-shell` y `nodalix-greeter-theme` en una
transacción declarada, archivando los datos de Hyprland y retirando sus paquetes. Las integraciones
pasan a su paquete propio. El actualizador original de 0.2.4 necesita el puente
`nodalix-migrate-gnome`: primero instala el nuevo actualizador verificado y
continúa la actualización completa. No usa sobrescrituras generales.

La actualización instala las dependencias con una actualización completa de
Arch, habilita GDM y CUPS para el siguiente arranque y guarda la configuración
anterior en `/var/lib/nodalix-updater/migrations/gnome-0.3.0/files`. Al finalizar,
Ajustes exige «Reiniciar ahora» y bloquea otras actualizaciones hasta el siguiente
arranque. No hay cuenta atrás ni se reinicia automáticamente.

El instalador nuevo activa GDM para el siguiente inicio. No reinicia el gestor
de acceso ni termina la sesión actual. En GNOME, ejecutar `nodalix-gnome-migrate`
guarda los estados y la configuración de systemd, desactiva la shell/acento
antiguos y bloquea los servicios de shell y fondos. Enlace móvil sigue en su backend.

Los portales preferidos son `gnome;gtk;`. GTK sigue siendo un backend válido,
con funciones distintas; se retira el backend Hyprland.
No se aplican variables Ozone/GDK globales ni reglas de ventana por aplicación.

## Iconos y aplicaciones

El tema `Nodalix-Adwaita-Symbols` sustituye símbolos de controles y paneles de
Ajustes por los originales de GNOME/Adwaita; hereda Colloid para aplicaciones.
`nodalix-shell-symbols` crea una variante de usuario que hereda su tema actual.
El gestor `nodalix-app-icons` excluye los paneles internos de Ajustes y los
iconos simbólicos; retira solo las entradas que él creó y que siguen intactas.
Así no sustituye los nombres de los controles por rutas de archivos de aplicaciones.

Enlace móvil usa `com.nodalix.PhoneLink` tanto como identificador GTK de su interfaz
como nombre de su entrada desktop. El nombre D-Bus del backend permanece
`com.gabriel.iphonebridge`, para conservar los clientes y el servicio existentes.
Nodalix Connect observa ese servicio y abre su aplicación; no implementa Bluetooth.

## GLocalSend y Nautilus

La copia mantenida en `gnome/extensions/glocalsend@donnybeelo.github.com` conserva
español, favoritos por fingerprint, orden de favoritos, ajustes de inicio y
compatibilidad de orientación con GNOME 51. No contiene certificados, claves ni
configuración personal. Se instala desde el paquete GNOME junto a Nodalix Connect,
Dash to Dock 109, Blur my Shell 74, Tiling Assistant 55 y Rounded Window Corners Native
51.0. Hanabi se incluye en el paquete de fondos. Solo las copias de usuario del perfil que no declaran GNOME 51 se archivan
al primer login; se conservan las copias compatibles y las extensiones adicionales.
Se conservan los ajustes existentes; las cuentas nuevas reciben el aspecto del
perfil actual, sin fijar nombres de monitores de este equipo.

La copia incluye un puente D-Bus pequeño, `com.nodalix.LocalSend1`, que devuelve
estado y dispositivos ya descubiertos. Nautilus utiliza ese puente y lanza un
proceso separado que transmite archivos por bloques. HTTPS verifica el fingerprint
del receptor; las carpetas se empaquetan temporalmente, sin archivos enlazados.
Un rechazo parcial no se presenta como un envío completo. El receptor y el
intercambio de texto actuales de GLocalSend siguen funcionando dentro de la
extensión: separar también esa parte del compositor sigue siendo trabajo futuro.

Los módulos de las extensiones cargadas se mantienen en memoria durante la
sesión. Para cargar cambios de código, cerrar sesión normalmente y volver a entrar.
No reiniciar GNOME Shell en una sesión Wayland con aplicaciones abiertas.

## Validación antes de publicar

Ejecutar las pruebas de `tests/`, validar los esquemas y construir los paquetes.
Las pruebas del puente D-Bus, sender y paquetes no sustituyen estas pruebas reales:

- Inicio desde GDM y desde la ISO en VM, instalación completa y actualización de
  un sistema con la shell antigua instalada.
- Restauración, maximización, movimiento y tamaño de aplicaciones GTK, Qt,
  Electron, XWayland y navegadores; agrupación e iconos de la suite instalada.
- File chooser, compartir pantalla, capturas, OpenURI y arrastrar archivos.
- Nautilus → iPhone favorito, PIN, archivos grandes, recepción, cancelación,
  LocalSend al inicio y alternar activación de la extensión.
- Llamadas y notificaciones iPhone, batería y audio AirPods, iCloud Drive,
  calendarios y servicios externos del usuario.

## Reversión

Usar el respaldo específico del equipo antes de restaurar archivos del sistema.
Para el tema de símbolos, seleccionar de nuevo el tema de aplicaciones anterior.
Para las extensiones, restaurar su carpeta respaldada y volver a iniciar sesión;
no restaurar toda la base dconf, porque podría sobrescribir cambios posteriores.

Para volver a una sesión Hyprland explícita habría que reinstalar sus paquetes
y restaurar sus configuraciones y unidades respaldadas.
No activar Quickshell mientras se permanezca en GNOME.
La configuración histórica del instalador se obtiene del tag `0.2.4` solo en las pruebas;
no entra en la nueva ISO ni en una instalación oficial GNOME.

## Ajustes nativos y fondos animados

Las operaciones de actualizaciones continúan aunque se cierre Ajustes. Incluyen
Nodalix, paquetes de repositorios Arch (también el kernel), apps instaladas mediante
Nodalix Apps, Flatpak cuando está instalado y firmware cuando fwupd está instalado.
La autenticación de administrador aparece para las operaciones privilegiadas.
Actualizar todas las apps no instala apps ausentes ni reinicia Quickshell en GNOME.

Seleccionar un vídeo habilita el reproductor y lo configura sin audio, con pausa
al maximizar o entrar en pantalla completa y con batería baja. Seleccionar un fondo
estático o «Usar el fondo estático» deshabilita la reproducción. El logo de GDM y
la detección de la nueva extensión se ven al volver a iniciar sesión normalmente.

Fuentes, revisiones, construcción y pruebas:
[`gnome/control-center/UPSTREAM.md`](../gnome/control-center/UPSTREAM.md).
Las recetas y el instalador distinguen el proveedor Nodalix de Ajustes del paquete
stock. El instalador permite el conflicto declarado para sustituir ese proveedor;
no sobreescribe archivos de otros paquetes.

### LocalSend desde Ajustes

Compartir → LocalSend usa el mismo servicio GLocalSend que los controles rápidos
y Nautilus. Comparte nombre, carpeta de recepción, activación al iniciar sesión,
temporizador, aceptación automática y favoritos por fingerprint. El menú de
Nautilus incluye «Ajustes de LocalSend». Se mantiene la identidad TLS existente;
no se inicia un segundo receptor. Los módulos nuevos se cargan al volver a
iniciar sesión normalmente. Acerca de muestra el logo Nodalix a 96 píxeles de
altura y permite reducir la imagen original de alta resolución.

### Impresión y fondos en la página Apariencia

La base GNOME instala `cups` y `cups-pk-helper`, y habilita `cups.socket`.
Los fondos animados se seleccionan directamente en Apariencia, debajo de los
estáticos, con miniaturas de 144×108 y los mismos bordes e indicador de selección.
El estado distingue reproducción, pausa y activación pendiente del próximo
inicio de sesión. La selección queda persistida y aplica un fotograma nítido
a resolución completa mientras se carga el reproductor. Las miniaturas pequeñas
se usan solo en la galería. Elegir un fondo estático cancela también
una activación pendiente. Se conserva el resto de extensiones del usuario.

### Tamaño del logo en GDM

El inicio de sesión usa un recurso propio de 64×64, `nodalix-login-logo.svg`,
generado con la imagen original incrustada. GDM carga las imágenes a su tamaño
intrínseco; el PNG original de 1254×1254 no debe configurarse directamente como
logo del inicio de sesión. Se comprobaron las dimensiones con el cargador St de
GNOME 51 a escala normal y doble. Actualizar la configuración dconf no exige
reiniciar GDM ni interrumpir la sesión activa.

### Arranque y desbloqueo de fondos animados

La colección incluye fotogramas de respaldo a 3840×2160 en
`/usr/share/nodalix/wallpaper-stills`; el escritorio ya no amplía la miniatura de
480 píxeles. El reproductor no añade demora de inicio, detecta sus ventanas cada
100 ms y usa una transición de 150 ms.

La extensión admite `user` y `unlock-dialog`. Con la animación oculta en la
pantalla de bloqueo, mantiene el reproductor y su último fotograma en pausa;
al desbloquear reevalúa la pausa automática y conserva una pausa manual previa.
Al deshabilitar la extensión o cerrar sesión sí elimina el reproductor y sus
señales. Esta actualización de módulos y modos se carga con el próximo inicio
de sesión normal; no se reinicia GNOME Shell ni se interrumpe la sesión activa.
La prueba en GNOME 51 aislado conservó el PID y las fuentes de los clones,
confirmó reproducción en la comprobación a unos 151 ms tras desbloquear,
conservó la pausa manual y detuvo el proceso al desactivar.

### Fondo en la pantalla de acceso

`nodalix-settings` instala una extensión específica de GDM,
`nodalix-login-background@getnodalia.com`, compatible con GNOME 51. Añade un
fondo no interactivo detrás de los controles de acceso, con el desenfoque de
90 píxeles y brillo 0,65 que GNOME usa en el bloqueo. No altera la autenticación,
los campos, el logo, las cuentas ni el tema de GNOME.

El perfil dconf de GDM habilita únicamente esta extensión para el acceso,
conservando las demás extensiones que estén configuradas en ese perfil.
`/etc/dconf/db/gdm.d/91-nodalix-login-background` guarda las URI de la imagen.
La colección proporciona un fotograma 4K estático del fondo animado; GDM no
inicia el reproductor de vídeo. En el equipo se configuró el fondo actual
Mystic Lights, accesible públicamente en `/usr/share/nodalix/wallpaper-stills`.
Los cambios posteriores del fondo del usuario no se copian automáticamente
al perfil de acceso, que es compartido por todas las cuentas del equipo.

Se probó el modo GDM real en un compositor aislado y se comprobó visualmente
que el fondo aparece detrás de los controles y del logo compacto. La sesión
normal no habilita esta extensión y el servicio GDM no se reinicia: la
configuración se ve al volver a la pantalla de acceso normalmente.


## Sesión y migraciones

El wrapper de sesión solo prepara el entorno y ejecuta GNOME. No ejecuta el
migrador de usuario. El autostart de fase Applications ejecuta el trabajador
Python después del inicio gráfico; un fallo no puede devolver el usuario a GDM.
Un bloqueo exclusivo evita ejecuciones simultáneas, el límite es 120 segundos y
el siguiente login reintenta los pasos incompletos. El marcador es
`$XDG_STATE_HOME/nodalix/migrations/gnome-0.3.0-v2.done` (por defecto `~/.local/state`).
El registro se rota al superar 1 MiB y conserva una copia anterior. Cada versión
tiene un único directorio de respaldo; no crea copias fechadas en cada login.

El migrador de sistema conserva sus comprobaciones de propiedad y hashes de los
archivos sin propietario del instalador 0.2.4. Nunca usa una sobrescritura general
ni elimina aplicaciones por contener `hypr` en su nombre: Hylki queda intacto.
Las preferencias GNOME explícitas prevalecen sobre defaults. Los atajos nuevos se
fusionan con los personales, el teclado deriva de XKB/keymap/locale y los handlers
XDG elegidos por el usuario se mantienen. Solo el launcher ChatGPT con el Exec
antiguo exacto se archiva; los launchers personales distintos se conservan.

## Mutter, blur y extensiones

`gnome-rounded-blur` procede de `kancko/gnome-rounded-blur`, commit
`c0d67c886ac0b54fedaddf75817e85264d16322e`, con `libmutter-51`,
`mutter-clutter-51` y `mutter-cogl-51`. Los paquetes requieren Mutter 51 y
GNOME Shell 51 y excluyen 52. Una actualización de versión mayor exige revisar
el código, cambiar los límites ABI y volver a ejecutar compilación y pruebas de
Shell/GDM. Un conflicto ABI falla explícitamente, en lugar de cargar una librería
compilada para otra versión. Blur My Shell detecta el soporte real en runtime;
Nodalix no fuerza `rounded-blur-found=true`.

Los schemas privados se cargan desde el directorio de cada extensión con
`Gio.SettingsSchemaSource`. El Dock inferior usa autohide e intellihide,
44 px, varios monitores y ocultación en fullscreen. No fuerza preferencias
explícitas durante una actualización.

`nodalix-secondary-bars@getnodalia.com` crea únicamente actores de barra con
Actividades y reloj. La barra principal conserva Quick Settings, calendario,
GLocalSend y Enlace móvil. No construye otro `Panel.Panel`, ni mueve indicadores.
Los cambios de monitor, escala y altura reconstruyen las barras; chrome reserva
workarea y respeta fullscreen. El blur propio usa Shell.BlurEffect. Las llamadas
privadas a layoutManager están encapsuladas y protegidas por GNOME 51. No se
incluye Top Bar All Monitors con metadata parcheada.

La prueba `tools/test-gnome51-shell.py` arranca GNOME 51 headless en un HOME y
bus privados, con dos monitores y ciclos repetidos de enable/disable. La única
extensión que permite Eval es temporal, creada dentro del entorno de pruebas;
ningún paquete habilita unsafe mode. Phone Link mantiene su backend independiente
ANCS/oFono/HFP; Nodalix Connect solo proporciona la interfaz de Quick Settings.

## Fondos, login y cursores

Las rutas canónicas son `backgrounds/nodalix/static` y `animated` bajo
`/usr/share`. `Estáticos` y `Animados` son enlaces de compatibilidad, no copias.
El catálogo GNOME contiene los siete JPG. Hanabi conserva su upstream y usa
VA-API (`gst-plugin-va`) sin preferir clappersink; `libva-utils` es diagnóstico.

Al seleccionar vídeo, el servicio obtiene un poster por hash de identidad,
tamaño y mtime. Los vídeos empaquetados reutilizan sus imágenes verificadas;
los personales usan ffmpeg con tiempo limitado. Se actualizan background,
background-dark y screensaver. El cache no regenera en cada login.

La publicación en GDM envía bytes JPEG a un helper mediante polkit; root no abre
rutas del HOME. El helper limita tamaño y tiempo, recodifica el JPEG y publica
atómicamente en `/var/lib/nodalix/login/background.jpg`. Si la autorización no
se concede, permanece el último fondo del sistema y la sesión continúa. El
fallback empaquetado es `/usr/share/backgrounds/nodalix/login/nodalix-login-fallback.jpg`.
GDM carga esa ruta explícita en todos los monitores con cover, blur y brillo
reducido. El acceso privado al grupo del greeter está limitado a la extensión
GNOME 51 y debe revalidarse al cambiar de versión.

El cursor hereda Adwaita y hicolor y valida enlaces XCursor estándar durante el
paquetizado. El tamaño inicial es 24. No hay overrides CSS para forzar controles
internos de libadwaita; la disposición de botones es `:minimize,maximize,close`
y cada aplicación puede ocultar botones según la función de su ventana.

## Atajos y terminal

| Atajo | Acción |
|---|---|
| Super+Enter | Terminal XDG (`xdg-terminal-exec`, Ghostty si no hay elección previa) |
| Super+B / Super+M | Navegador / correo según handler XDG actual |
| Super+Q / Super+F | Cerrar / fullscreen |
| Super+Shift+3 / Super+Shift+4 | Captura completa / interfaz de selección GNOME |
| Super+N | Calendario y message tray |

Focus follows mouse usa sloppy y no auto-raise si el usuario no eligió otra
opción. Ghostty recibe CSD GTK, titlebar con tabs arriba y copy performable para
Ctrl+C: sin selección sigue enviando SIGINT. Las líneas gestionadas se deduplican
y el resto se conserva con respaldo inicial `config.pre-gnome-030`.

Super+C/V/Z requiere la función opcional keyd; no se activa globalmente desde un
paquete ni instala IDs universales. `nodalix-keyboard-remap` detecta teclados
físicos con las teclas de escritura necesarias, excluye ratones, ejes y dispositivos
virtuales, y muestra una configuración concreta para keyd 2.6.0. El administrador
puede revisarla en `/etc/keyd/` y habilitar keyd. Repetir la detección tras cambiar
teclado/receptor; otros keyd requieren validar su algoritmo de fingerprint.
No se usa `[ids] *` y Ctrl+C convencional permanece disponible.

## Legacy aislado y retirada futura

| Apariciones | Motivo y conservación |
|---|---|
| `updater/legacy-installer-0.2.4.json`, migradores y lista de paquetes retirados | Hashes, respaldo y transición desde versiones antiguas; retirar al cerrar el soporte de upgrades 0.2.x |
| `gnome/migration/` y pruebas de transición | Detección y archivo de configuraciones antiguas, sin ejecutar sus programas |
| `docs/releases/`, `docs/baseline/` | Historia y evidencia de versiones publicadas; no se empaquetan |
| Fixture `git archive 0.2.4` de CI | Instala el overlay original para comprobar una migración real |
| `co.hyprlab.Hylki.desktop` | Aplicación actual legítima, ajena al compositor antiguo |

Se retiraron `shell/`, paquetes QuickShell/Hymission/greeter/wallpaper-engine,
el overlay legacy y herramientas y tests exclusivos de aquella UI. iCloud,
vdirsyncer, organización e iconos viven en `integrations/`; el antiguo receptor
LocalSend no se empaqueta ni compite con GLocalSend.
