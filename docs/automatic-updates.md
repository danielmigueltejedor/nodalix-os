# Actualizaciones automáticas en Settings

La página Actualizaciones incluye cuatro interruptores independientes:
Nodalix, sistema/kernel, Flatpak y firmware. Nodalix, sistema y Flatpak se
comprueban a diario; firmware, semanalmente, con un retardo aleatorio de hasta
30 minutos. Los temporizadores persistentes recuperan una ejecución pendiente
cuando vuelve a encenderse el equipo.

Al activar un interruptor se solicita autorización de administrador mediante
el diálogo del sistema. La autorización queda registrada antes de iniciar el
temporizador. No se guarda ninguna contraseña. Las ejecuciones posteriores y
los reinicios del equipo no vuelven a solicitarla. El usuario que autorizó
puede desactivar ese interruptor sin contraseña; no se permite que otro usuario
retire su autorización. Desactivar detiene la programación futura y no interrumpe
una transacción que ya esté instalando paquetes.

Flatpak incluye tanto instalaciones del sistema como las del usuario que lo
autoriza. Se desactiva el temporizador de usuario antiguo para evitar duplicados.
Las aplicaciones AUR continúan con revisión manual. Los interruptores reflejan
los temporizadores reales, incluso al reabrir Settings. Cancelar el diálogo
o fallar la configuración restaura el estado real; un fallo parcial se revierte.

Los trabajos ejecutan operaciones fijas, con bloqueo común entre las
actualizaciones privilegiadas. Las comprobaciones de espacio, batería, bloqueo
de paquetes y reinicio obligatorio se comparten con la actualización manual.
Las actualizaciones de sistema mantienen la protección Btrfs cuando está
activada y verifican después la base de datos de paquetes. Los cambios de kernel
o paquetes críticos dejan una recomendación de reinicio. Nunca se reinicia el
equipo automáticamente. El estado está en `/var/lib/nodalix-updater/automatic-updates.json`.

La preparación prueba el diálogo de Settings contra un servicio simulado y
la autorización/programación con un gestor simulado. Los paquetes se compilan
en el PC; no se activan actualizaciones automáticas sobre el sistema anfitrión.
La ejecución real privilegiada y su persistencia tras arrancar siguen siendo
parte de la validación de instalación y actualización de la versión.
