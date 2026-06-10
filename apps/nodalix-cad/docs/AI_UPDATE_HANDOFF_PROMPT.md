# Prompt — Actualizar handoff de sesión

Copia y pega esto al **final** de una sesión de IA (Cursor, ChatGPT, Codex, etc.) cuando quieras dejar el proyecto documentado para la siguiente sesión.

---

## Prompt (español)

```text
Actualiza el archivo docs/AI_SESSION_HANDOFF.md con todo lo hecho en esta sesión.

Reglas:
- No borres información importante de sesiones anteriores; integra y condensa si hace falta.
- Actualiza las secciones 2, 9–16 y añade una entrada nueva al final usando la plantilla "Session update template" (sección 17).
- Incluye: objetivo, cambios, por qué, archivos tocados, decisiones técnicas, validación (cargo check/test/fmt/clippy/build), hash sha256 de target/release/nodalix-cad y ~/.local/bin/lixcad si se instaló, bugs corregidos, bugs conocidos, deuda técnica, siguiente fase recomendada.
- Ruta del proyecto SIEMPRE: /home/dani/Proyectos/nodalix-os/apps/nodalix-cad (nunca /home/dani/Projects/...).
- No modifiques código salvo que sea necesario para corregir el handoff.
- Mantén el documento legible para pegarlo entero en una sesión nueva.
```

---

## Prompt (English, optional)

```text
Update docs/AI_SESSION_HANDOFF.md with everything done in this session.

Rules:
- Do not remove important prior session notes; merge and condense if needed.
- Refresh sections 2, 9–16 and append a new entry using section 17's template.
- Include: goal, changes, rationale, files touched, technical decisions, validation (cargo check/test/fmt/clippy/build), sha256 hashes for release binary and ~/.local/bin/lixcad if installed, fixed bugs, known bugs, tech debt, recommended next phase.
- Project path MUST be: /home/dani/Proyectos/nodalix-os/apps/nodalix-cad (never /home/dani/Projects/...).
- Do not change application code unless required to fix the handoff doc.
- Keep the doc suitable for pasting into a fresh AI session.
```

---

## Cuándo usarlo

- Al cerrar una **fase grande** (p. ej. 3.18 Layers).
- Al cerrar un **checkpoint de fase** (p. ej. **3.30 LixCAD 2D Core Tools Baseline**): `cargo clean`, full test suite, install, actualizar hash, añadir sección “Fase 3.30” + smoke checklist + deuda clasificada — **sin features nuevas**.
- Antes de cambiar de máquina o de herramienta de IA.
- Cuando notes que el contexto del chat ya está “sucio” o contradictorio.
