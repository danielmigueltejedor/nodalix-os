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
- `nodalix-shell`, Hymission, greetd y el motor de fondos antiguo son opcionales.
  La shell, los fondos y los cambios de acento heredados solo se ejecutan bajo Hyprland.

Actualizar la shell antigua y las integraciones en **una misma transacción**:
los servicios compartidos cambian de propietario. El actualizador y el instalador
incluyen los componentes opcionales que ya estén instalados. Instalar únicamente
`nodalix-integrations` junto a una versión antigua de `nodalix-shell` puede producir
conflictos de archivos. No usar `--overwrite` para resolverlos.

El instalador nuevo activa GDM para el siguiente inicio. No reinicia el gestor
de acceso ni termina la sesión actual. En GNOME, ejecutar `nodalix-gnome-migrate`
guarda los estados y la configuración de systemd, desactiva la shell/acento
antiguos y bloquea los servicios de shell y fondos. Enlace móvil sigue en su backend.

Los portales preferidos son `gnome;gtk;`. GTK sigue siendo un backend válido,
con funciones distintas; el backend Hyprland recibe una condición de sesión.
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
configuración personal. No se instala sobre la versión del usuario desde el paquete
OS: revisar `UPSTREAM.md` antes de preparar el fork redistribuible.

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

Para volver a una sesión Hyprland explícita, restaurar sus unidades respaldadas,
desbloquear shell/fondos y habilitar los servicios que estuvieran habilitados antes.
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
