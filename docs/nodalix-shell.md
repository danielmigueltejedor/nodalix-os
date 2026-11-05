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

## Vista previa de Nodalix OS 0.5.0 (GNOME Shell 51.1)

- GNOME Shell 51.1 fijada al commit upstream
  `33da06854e229c256afd5872fedd83c28cbfa176` y Mutter 51.
- `tools/prepare-nodalix-shell.py` integra el arranque y el registro de módulos
  en los recursos compilados de GNOME.
- El paquete incluye el dock, las barras en todos los monitores, Blur My
  Shell, Tiling Assistant, el mosaico automático, el menú global, las esquinas
  redondeadas, la búsqueda web y los dos módulos de cursor. Utiliza las implementaciones
  mantenidas en Nodalix con sus esquemas y licencias.
- El arranque nativo se ejecuta tras crear la interfaz. No se carga en
  GDM ni en la configuración inicial.
- **Enlace móvil** es un módulo interno de Nodalix Shell con un acceso en
  Quick Settings y observación del servicio D-Bus existente; no replica el
  servicio Bluetooth ni el backend de Phone Link.
- La Shell carga sus módulos desde el paquete. Conserva sus UUID y esquemas
  para leer los ajustes existentes y evita cargar una segunda copia como
  extensión. El gestor estándar sigue atendiendo las extensiones opcionales.
- `nodalix-shell-preview` proporciona `gnome-shell` y entra en conflicto
  **solo** con el paquete binario stock. No usa `replaces`, no aparece en
  `release/components.json` y no se instala con el actualizador habitual.

### Construcción y comprobación

Ejecutar exclusivamente en una VM/chroot Arch de pruebas, con los paquetes
necesarios y GNOME 51/Mutter 51 compatibles:

```bash
python3 -m unittest discover -s tests -p 'test_nodalix_shell.py' -v
bash tools/build-nodalix-shell-preview.sh --prepare-only
bash tools/build-nodalix-shell-preview.sh
```

El constructor deja la candidata `nodalix-shell-preview` en `dist/packages`.
La actualización de desarrollo se prueba junto con los paquetes de GNOME,
Ajustes y fondos animados de la misma compilación. El actualizador estable
conserva su selección de paquetes. Antes de probar una
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

Nodalix OS mantiene un manifiesto de componentes versionados. La **futura**
integración estable solo debe incorporarse a ese manifiesto cuando el fork
haya pasado pruebas reales de compilación, clean install, migración,
rollback y pruebas de VM contra la misma versión de GNOME/Mutter.

No ejecutar actualizaciones mayores de GNOME automáticamente en el fork:
cada salto de ABI pasa por portado de módulos, comparativa de parches,
pruebas deterministas, paquete reproducible y lanzamiento de Nodalix.

### Alcance de la vista previa

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
fijado. Nodalix Shell ignora las copias instaladas de los módulos incorporados
en la lista de extensiones. Las copias personales y sus datos se conservan
para que sigan disponibles al volver a GNOME estándar.

## Definición de listo para uso diario

1. Compilación en Arch con GNOME 51.1 y tests de Meson aprobados.
2. Inicio de sesión GDM, bloqueo y retorno de suspensión probados en VM.
3. Actualizador, pacman, instalación limpia y rollback probados con snapshots.
4. Ausencia de extensiones Nodalix duplicadas y preservación de preferencias.
5. Sin regresiones en Wayland, accesibilidad, input, XWayland ni captura de pantalla.
6. Cada función nativa integrada queda sujeta a pruebas de Nodalix OS y
   se entrega únicamente en el lanzamiento de Nodalix correspondiente.

**Validación local.** GNOME Shell 51.1 compiló y arrancó en una sesión Wayland
aislada con dos monitores virtuales, trece módulos de usuario activos y una sola entrada
de Enlace móvil. Quedan por probar el inicio real con GDM, las transiciones
de bloqueo y suspensión y el paquete en una instalación completa.
