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

## Primer prototipo funcional (opt-in, GNOME Shell 51.1)

- GNOME Shell 51.1 fijada al commit upstream
  `33da06854e229c256afd5872fedd83c28cbfa176` y Mutter 51.
- `tools/prepare-nodalix-shell.py` añade `js/nodalix/main.js` y
  `js/nodalix/connect.js` al GResource de GNOME.
- El arranque nativo se ejecuta tras crear la interfaz. No se carga en
  GDM ni en la configuración inicial.
- **Enlace móvil** es un módulo interno de Nodalix Shell con un acceso en
  Quick Settings y observación del servicio D-Bus existente; no replica el
  servicio Bluetooth ni el backend de Phone Link.
- El gestor de extensiones de **esta compilación** ignora el UUID
  `nodalix-connect@getnodalia.com` para evitar la doble activación.
  Los ajustes GNOME del usuario no se modifican: el paquete estándar
  vuelve a reconocer el UUID al revertir.
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

El constructor no instala el paquete en la máquina. Antes de probar una
sesión, guardar snapshot. En la VM, hacer una instalación explícita del
paquete de prueba, iniciar sesión desde GDM, comprobar Quick Settings,
Wayland, bloqueo/desbloqueo, suspender/reanudar, varios monitores y registros.
Probar también una VM sin el servicio Phone Link y otra con el UUID antiguo
habilitado en dconf para detectar duplicados.

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

### Migración propuesta de extensiones

| Orden | Función | Estrategia |
|---|---|---|
| 1 | Enlace móvil (Nodalix Connect) | **Integrado en este prototipo** |
| 2 | Cursor de arranque, agitar puntero | Convertir lógica propia a módulos nativos |
| 3 | Mosaico automático | Portar junto a sus esquemas y reglas de ventanas |
| 4 | Dock y multimonitor | Diseñar superficie nativa, no copiar íntegramente Dash to Dock |
| 5 | Menú global | Integrar servicio y políticas por aplicación |
| 6 | Blur, esquinas y fondos | Adaptar shaders/efectos a Mutter y al pipeline probado |
| 7 | LocalSend, Tiling Assistant y otros | Auditar dependencias, código y licencias antes del port |

No convertir código de terceros a código interno sin revisar licencias y
atribución. Funciones de seguridad, portales, GDM y compositor siguen siendo
upstream mientras no se demuestre una razón para bifurcarlas.

## Definición de listo para uso diario

1. Compilación en Arch con GNOME 51.1 y tests de Meson aprobados.
2. Inicio de sesión GDM, bloqueo y retorno de suspensión probados en VM.
3. Actualizador, pacman, instalación limpia y rollback probados con snapshots.
4. Ausencia de extensiones Nodalix duplicadas y preservación de preferencias.
5. Sin regresiones en Wayland, accesibilidad, input, XWayland ni captura de pantalla.
6. Cada función nativa integrada queda sujeta a pruebas de Nodalix OS y
   se entrega únicamente en el lanzamiento de Nodalix correspondiente.

**Estado:** infraestructura y primer módulo listos para revisar y compilar;
no hay compilación, arranque real ni sustitución del escritorio verificados
en esta propuesta inicial.
