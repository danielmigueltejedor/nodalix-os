# Organización de carpetas de Nodalix

Nodalix aplica esta política al iniciar la shell, al cambiar de idioma y mediante una revisión periódica por usuario.

| Contenido | Ubicación |
| --- | --- |
| Documentos y trabajo de Codex | Directorio XDG de documentos; Codex mantiene su subcarpeta |
| Fuentes y proyectos | Documentos/Proyectos (es) o Documents/Projects (en) |
| Repositorios heredados Nodalix, src y repositorios autónomos visibles | Subcarpetas de Proyectos |
| Datos privados de aplicaciones Nodalix | XDG_DATA_HOME/nodalix |
| Ajustes | XDG_CONFIG_HOME |
| Estado, historial de migraciones e iconos | XDG_STATE_HOME/nodalix |
| Compilaciones y temporales de Nodalix | XDG_CACHE_HOME/nodalix |
| Transferencias de Enlace móvil | XDG_CACHE_HOME/nodalix/phone-link/transfers, permisos privados |

Se respetan las rutas XDG personalizadas y las carpetas desactivadas. No se clasifican documentos por su contenido. Los repositorios enlazados mediante worktrees se conservan en su ruta. Solo se eliminan carpetas heredadas de escritorio o transferencias cuando están vacías y no son puntos de montaje.

Cada migración registra origen y destino y conserva la configuración XDG anterior. Antes de mover se comprueban conflictos: nunca se reemplaza un archivo existente. Los cambios de idioma conservan enlaces de compatibilidad ocultos para aplicaciones que recuerdan las rutas antiguas. Estos enlaces no son una segunda copia de los documentos.

El idioma soportado actualmente es español o inglés, como la shell. Las carpetas del sistema (/usr, /etc, /var) mantienen sus nombres estándar. Las rutas técnicas de datos, estado y caché no cambian de idioma.

`nodalix-user-layout` muestra el plan sin modificar archivos; `--apply` lo ejecuta. `--compatibility` conserva también enlaces de los proyectos heredados. Las unidades de usuario `nodalix-user-layout.timer` y `nodalix-app-icons.timer` realizan la revisión periódica.
