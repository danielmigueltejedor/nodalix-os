# Migrar de Nodalix 0.2.4 a 0.3.0

La release 0.3.0 incluye una ISO instalable, los paquetes del escritorio y el puente de migración. Guarda tu trabajo antes de actualizar; al terminar tendrás que reiniciar.

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

Para actualizar desde 0.2.4 con la versión publicada:

```sh
curl -fL https://github.com/danielmigueltejedor/nodalix-os/releases/download/0.3.0/nodalix-migrate-gnome -o nodalix-migrate-gnome
sudo python3 nodalix-migrate-gnome
```

El puente descarga el manifiesto y el actualizador de la release 0.3.0 y continúa
con la actualización completa. Si aún no está publicada, falla sin instalar nada.
Los candidatos locales requieren administración y nunca se descubren como versiones
estables. Si el actualizador exige firmas, el candidato debe cumplir esa política.

La actualización verifica hashes antes de instalar, prepara GNOME mediante una
actualización completa de Arch y reemplaza los paquetes declarados. Los archivos originales del instalador 0.2.4 se reconocen por su hash y se respaldan antes de retirarlos. Los archivos
ajenos o personalizados sin propietario producen un error; no hay sobrescrituras generales.
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
instalable y se publica únicamente después de la validación de la release. El ensayo virtual no comprueba
impresoras o móviles físicos.
