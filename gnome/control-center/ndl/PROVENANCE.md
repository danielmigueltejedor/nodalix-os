# Hojas NDL (Nodalia Design Language)

Copia fijada de `packages/gtk/crate/src/css/` de
https://github.com/danielmigueltejedor/nodalia-design (MIT), versión
`0.1.0-alpha.1`, commit `58724b0258c5f3258bbbe7bdd81b9d3801096be0`. La misma
copia usa la Tienda (repo `tienda`, `src/ndl/`).

No se editan a mano: están generadas por `@nodalia/gtk`. Para actualizarlas,
copia las ocho hojas del commit nuevo, actualiza este fichero y `SHA256SUMS`,
y ejecuta `sha256sum -c SHA256SUMS` desde este directorio. Lo propio de
Nodalix OS va en `nodalix-overrides.css`, que se carga después y sí es nuestro.
