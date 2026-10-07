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
51.0. Hanabi se incluye en el paquete de fondos. Las copias de usuario se archivan
antes del primer arranque para evitar que oculten las versiones del sistema.
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
La configuración histórica de greetd/Hyprland se conserva en `iso/legacy-overlay`;
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
