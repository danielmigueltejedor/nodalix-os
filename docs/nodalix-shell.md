# Nodalix Shell — GNOME nativo, mantenido por Nodalix

## Visión

Nodalix Shell es una distribución **mantenida de GNOME Shell**, basada en
GNOME Shell y Mutter, no un compositor nuevo ni una colección de extensiones
independientes. Las funciones propias de Nodalix se integran en los módulos
JavaScript que compila GNOME Shell como GResources. La versión de Nodalix OS
fija la versión de la shell y el ABI de Mutter.

**Objetivo:** controlar diseño, comportamiento, regresiones y ritmo de
actualización en cada lanzamiento de Nodalix, sin depender de que extensiones
de terceros se adapten a una actualización imprevista de GNOME.

## Integración de Nodalix OS 0.5.0 (GNOME Shell 51.1)

- GNOME Shell 51.1 fijada al commit upstream
  `33da06854e229c256afd5872fedd83c28cbfa176` y Mutter 51.
- `tools/prepare-nodalix-shell.py` integra el arranque y el registro de módulos
  en los recursos compilados de GNOME.
- El paquete incluye el dock, las barras en todos los monitores, Blur My
  Shell, Tiling Assistant, el mosaico automático, el menú global, las esquinas
  redondeadas, la búsqueda web y los dos módulos de cursor. Utiliza las implementaciones
  mantenidas en Nodalix con sus esquemas y licencias.
- El arranque nativo se ejecuta tras crear la interfaz. En GDM solo activa
  los módulos compatibles con la sesión de acceso, incluido su fondo.
- **Enlace móvil** es un módulo interno de Nodalix Shell con un acceso en
  Quick Settings y observación del servicio D-Bus existente; no replica el
  servicio Bluetooth ni el backend de Phone Link.
- La Shell carga sus módulos desde el paquete. Conserva sus UUID y esquemas
  para leer los ajustes existentes y evita cargar una segunda copia como
  extensión. El gestor estándar sigue atendiendo las extensiones opcionales.
- `nodalix-gnome-shell` proporciona `gnome-shell=1:51.1` y entra en conflicto
  con el paquete stock y la vista previa anterior. El manifiesto firmado,
  el instalador y la sesión live lo seleccionan como componente obligatorio
  de 0.5.0. La transacción instala también las versiones compatibles de
  los paquetes de escritorio, Ajustes y fondos animados. Requiere reiniciar.
- El constructor antiguo `build-nodalix-shell-preview.sh` sigue disponible
  para pruebas explícitas; ese paquete no forma parte del manifiesto.

### Transición del acceso a la sesión (0.6.0)

GNOME Shell avisa a GDM de que la sesión está lista (`RegisterSession`) antes
de dibujar su primer fotograma. GDM retira entonces el acceso y, hasta que la
Shell pinta algo, solo se ve un fondo negro (después, el gris oscuro de
arranque de GNOME y una animación que compite con la carga de los módulos).
Nodalix Shell lo corrige en `gnome/nodalix-shell/transition.js`:

1. **Retiene el registro en GDM** hasta que la cubierta se ha pintado (hasta
   300 ms de espera si la sesión aún no puede pintar), de modo que la pantalla
   de acceso sigue visible hasta entonces. Si algo se atasca, el registro se
   envía igualmente a los 3 s y la cubierta se retira a los 15 s: un inicio de
   sesión nunca se queda esperando.
2. **El primer fotograma es el fondo del acceso**, con el mismo desenfoque y
   brillo que el greeter (`/var/lib/nodalix/login/background.jpg`, o el fondo
   de respaldo), en lugar del gris oscuro del sistema.
3. **Desblur**: cuando el escritorio está completo (módulos habilitados, con un
   máximo de 1,5 s), el desenfoque y el oscurecimiento se animan hasta cero
   durante 650 ms y la cubierta se desvanece sobre el escritorio real. La
   animación de arranque de GNOME se ejecuta debajo.
4. Hanabi (fondo de vídeo) espera a que termine la transición para no lanzar su
   proceso de reproducción mientras se anima.

`NODALIX_LOGIN_TRANSITION=0` en el entorno de la sesión (por ejemplo en
`~/.config/environment.d/`) lo desactiva y restaura el comportamiento de
GNOME. Con las animaciones desactivadas en Ajustes el desblur es instantáneo.

Cada paso queda en el diario con la marca de tiempo relativa al fin de la
construcción de la interfaz, para medir en hardware real:

```bash
journalctl --user -b -t gnome-shell | grep 'Nodalix login transition'
journalctl --user -b -t gnome-shell | grep -E 'Registering session with GDM|GNOME Shell started'
```

`tools/prepare-nodalix-shell.py` añade cuatro puntos de enganche en
`ui/layout.js` y uno en `ui/main.js` y falla si el código de GNOME difiere.

### Construcción y comprobación

Ejecutar exclusivamente en una VM/chroot Arch de pruebas, con los paquetes
necesarios y GNOME 51/Mutter 51 compatibles:

```bash
python3 -m unittest discover -s tests -p 'test_nodalix_shell.py' -v
bash tools/build-nodalix-shell.sh --prepare-only
bash tools/build-nodalix-shell.sh
```

El constructor deja la candidata `nodalix-gnome-shell` en `dist/packages`.
La actualización de desarrollo se prueba junto con los paquetes de GNOME,
Ajustes y fondos animados de la misma compilación. El canal estable publicado
permanece en 0.4.1 hasta publicar un nuevo manifiesto firmado. Antes de probar una
sesión, guardar snapshot. En la VM, hacer una instalación explícita del
paquete de prueba, iniciar sesión desde GDM, comprobar Quick Settings,
Wayland, bloqueo/desbloqueo, suspender/reanudar, varios monitores y registros.
Probar también una VM sin el servicio Phone Link y otra con el UUID antiguo
habilitado en dconf para detectar duplicados.

El paquete verifica sus recursos compilados y genera los esquemas de los módulos. Los tests de Meson de GNOME Shell requieren
la biblioteca privada `libmutter-test-51`, que el paquete habitual de
Mutter en Arch no incluye.

**Reversión en la VM:** desde TTY o sesión de recuperación, reinstalar el
paquete Arch `gnome-shell` compatible (con actualización coherente del
sistema), cerrar sesión y entrar de nuevo. Nunca sustituir binarios de la
sesión activa ni reiniciar GNOME Shell en caliente sobre Wayland.

### Política de actualización

Nodalix OS mantiene un manifiesto de componentes versionados. La candidata
incorpora la shell al manifiesto; su publicación requiere pruebas reales de
compilación, clean install, migración, rollback y VM contra la misma versión
de GNOME/Mutter.

No ejecutar actualizaciones mayores de GNOME automáticamente en el fork:
cada salto de ABI pasa por portado de módulos, comparativa de parches,
pruebas deterministas, paquete reproducible y lanzamiento de Nodalix.

### Módulos y compatibilidad

La compilación de desarrollo integra también Hanabi, la tarjeta multimedia
Now Playing Card, el cliente LocalSend que ya distribuye Nodalix y el fondo
de acceso de GDM. Hanabi, LocalSend y GDM comparten una copia del código con
sus paquetes de escritorio. Los enlaces para GNOME estándar sirven para
volver atrás conservando los mismos ajustes.

Tiling Assistant conserva la versión 56 de la sesión de referencia, con la
restauración tolerante a archivos dañados que mantiene Nodalix. El cierre
nativo cancela los refrescos pendientes de Hanabi antes de destruir los
actores del escritorio. Desactivar fondos animados durante la sesión sigue
restaurando los fondos estáticos.

Now Playing Card se obtiene del ZIP oficial de la versión 1.0.14, con SHA-256
fijado. Tras descomprimir el ZIP, `tools/patch-nowplaying-multimonitor.py`
extiende la extensión sin alterar licencia ni atribución: una sola fuente
MPRIS compartida alimenta actores independientes en el panel principal y en
cada barra secundaria de multi-monitors-bar, y el ocultador de controles
multimedia nativos actúa en todos los menús de fecha. El centro del panel
secundario admite varios indicadores a la vez (`_centerRow`). Nodalix Shell
ignora las copias instaladas de los módulos incorporados en la lista de
extensiones. Las copias personales y sus datos se conservan para que sigan
disponibles al volver a GNOME estándar.

## Perfil de escritorio incluido

La shell lleva empaquetada la configuración de referencia del escritorio de
Nodalix para no depender de ajustes manuales en cada instalación:

- `gnome/session/90-nodalix.gschema.override` fija los valores del sistema:
  tema oscuro, acento amarillo, esquinas activas desactivadas, aceleración
  plana del ratón, `Súper+Q`/`Alt+F4` para cerrar, `Súper+F` para pantalla
  completa, `Súper+Espacio` para abrir las aplicaciones (el cambio de teclado
  conserva la tecla de teclado dedicada) y espacios de trabajo en todos los
  monitores.
- `gnome/nodalix-shell/profile.json` contiene los valores de los módulos
  incorporados que difieren de sus esquemas (huecos de Tiling Assistant, menú
  global, paneles secundarios, Now Playing Card, desenfoque de panel, dock y
  ventanas emergentes, alias de LocalSend). El paquete lo valida contra los
  esquemas compilados y rechaza cualquier clave o valor desconocido.
- `gnome/migration/nodalix_profile.py` lo aplica una vez al iniciar sesión
  (`gnome-0.5.0-nodalix-shell-profile.done`). Solo escribe las claves que el
  usuario nunca ha definido, activa Now Playing Card si no se había
  desactivado y reintenta en el siguiente inicio si falta algún módulo.

El perfil excluye identificadores de monitores, huellas de LocalSend, rutas
personales, aplicaciones favoritas y fondos animados, que pertenecen a cada
usuario o a sus propios paquetes. Las tipografías son las predeterminadas de
GNOME (Adwaita Sans y Adwaita Mono).

## Definición de listo para uso diario

1. Compilación en Arch con GNOME 51.1 y tests de Meson aprobados.
2. Inicio de sesión GDM, bloqueo y retorno de suspensión probados en VM.
3. Actualizador, pacman, instalación limpia y rollback probados con snapshots.
4. Ausencia de extensiones Nodalix duplicadas y preservación de preferencias.
5. Sin regresiones en Wayland, accesibilidad, input, XWayland ni captura de pantalla.
6. Cada función nativa integrada queda sujeta a pruebas de Nodalix OS y
   se entrega únicamente en el lanzamiento de Nodalix correspondiente.

**Validación local.** GNOME Shell 51.1 compiló y arrancó en una sesión Wayland
aislada con dos monitores virtuales, trece módulos de usuario activos y una sola
entrada de Enlace móvil. Se comprobó también el fondo de GDM en dos monitores,
las barras y menús en tres monitores y el cambio de paquetes desde 0.4.1 con
retorno a GNOME estándar. Las pruebas de instalación y migración en QEMU
exigen ahora el paquete nativo y la ausencia de duplicados. El invitado de
pruebas realiza bloqueo/desbloqueo y suspensión por logind, y QMP verifica la
suspensión ACPI y el despertar. El ayudante de pruebas existe únicamente en
la VM desechable; nunca se distribuye en los paquetes ni en la ISO.
Los resultados de esas comprobaciones se registran en la CI de la candidata.
