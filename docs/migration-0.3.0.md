# Migrar de Nodalix 0.2.4 a 0.3.0

0.3.0 es por ahora una versión de desarrollo: este repositorio no crea una
release al subir cambios a main o gnome-first. No crear el tag 0.3.0 hasta que
se decida publicar: los tags numéricos sí activan el flujo de publicación.

El actualizador original de 0.2.4 no puede resolver la división de los servicios
ni instalar el escritorio nuevo en una sola transacción. Se proporciona un
puente pequeño que instala únicamente el actualizador 0.3.0 verificado y
continúa automáticamente con Nodalix Updater. No cambia la identidad del sistema
a 0.3.0 antes de que termine la instalación completa.

Para probar un candidato, descargar el artefacto `nodalix-0.3.0-candidate` de una
CI correcta, extraer todos sus paquetes y `nodalix-manifest.json` en una carpeta,
y ejecutar desde una copia del repositorio:

```sh
sudo python3 updater/nodalix-migrate-gnome --assets /ruta/al/candidato
```

Para la futura versión publicada, el mismo puente sin `--assets` descargará el
manifiesto y el actualizador de la release 0.3.0. Antes de su publicación falla
con un mensaje claro y no instala nada. Los candidatos locales requieren una
autorización explícita de administrador; nunca se descubren como actualizaciones
estables. Si el actualizador exige firmas, el candidato debe cumplir esa política.

La actualización verifica hashes antes de instalar, prepara GNOME mediante una
actualización completa de Arch y reemplaza los paquetes declarados. Los archivos
ajenos o sin propietario producen un error; no hay sobrescrituras generales.
Los registros están en `/var/lib/nodalix-updater`, junto con el historial y los
respaldos de GDM, greetd y AccountsService. No se reinicia GDM durante la operación.
Las cuentas que usaban Hyprland pasan a Nodalix/Wayland; se conservan otras sesiones
seleccionadas expresamente. La actualización retira Hyprland, Quickshell, greetd, su portal, Hymission y el
motor antiguo. La configuración histórica se guarda fuera de las rutas activas,
incluidas las copias personales de extensiones que ocultarían las nuevas versiones.
LocalSend conserva su identidad y favoritos; Ajustes, Nautilus y los controles
rápidos usan el mismo servicio. Se incluyen las siete extensiones del perfil GNOME
actual y sus esquemas, sin depender de descargas durante el primer inicio.

Al finalizar es obligatorio reiniciar. El servicio y Ajustes conservan el estado,
muestran «Reiniciar ahora» y bloquean otra actualización hasta el siguiente
arranque. En la primera sesión GNOME se guardan y desactivan las unidades antiguas
por usuario, sin interrumpir la sesión desde la que se realizó la actualización.
No se aplica un kernel distinto al elegido por el usuario: Arch actualiza los
paquetes del kernel que ya están instalados.

La CI compila todos los paquetes, valida las fuentes fijadas y hashes de los
16 vídeos (los más grandes se recomponen de partes sin recomprimir), e instala el candidato mediante el puente en un contenedor Arch.
La base 0.2.4 usa sus paquetes oficiales verificando los hashes e instala
sus dependencias reales y scripts heredados, incluido Hyprland y Quickshell. La migración candidata usa
las dependencias reales y los scripts de instalación, comprueba propiedad de
archivos, GDM, CUPS, identidad, conservación de datos y bloqueo por reinicio.
La CI también arranca ese sistema migrado en QEMU, inicia GNOME mediante GDM,
comprueba las siete extensiones, la migración de fondo/reloj/aplicaciones fijadas,
el servicio LocalSend y el reproductor animado. Se construye además la ISO
instalable como artefacto candidato, sin release. El ensayo virtual no comprueba
impresoras o móviles físicos.
