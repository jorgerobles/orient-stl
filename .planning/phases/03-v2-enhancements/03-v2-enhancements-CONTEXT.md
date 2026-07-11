# Context: Phase 3 — v2 Enhancements

**Capturado:** 2026-07-11
**Origen:** Discusión con usuario + deuda de Phase 2

## Domain

Mejora del scoring (height-weighted, hull+sphere samples) + overlay 3D interactivo (drag-to-rotate con score feedback) + hill-climb wizard button sobre candidato seleccionado.

## Phase Boundary (qué entra y qué NO)

### In scope
1. **Height-weighted scoring** — factor multiplicativo fijo k=0.5 en `compute.ts` (penalty * (1 + height_ratio * 0.5))
2. **Hull+sphere mode** — toggle hull / hull+sphere en UI; ~200 Fibonacci samples deduplicados con hull normals
3. **3D overlay drag-to-rotate** — modo sobre viewport: agarrar y rotar el modelo, score badge se actualiza en vivo
4. **Hill-climb wizard** — botón "varita mágica" en overlay; ejecuta hill-climb en Rust WASM desde la orientación actual

### Out of scope (diferido / cancelado)
- Multi-metric sort columnas ✗
- Heatmap en mesh ✗
- Side-by-side comparison ✗
- Circular yaw dial + geometry snap ✗
- Slider tilt ✗

## Decisiones clave

| Decisión | Valor |
|----------|-------|
| Height-weight k | 0.5 fijo, sin UI |
| Fibonacci samples | ~200, deduplicados con hull normals |
| Hill-climb runtime | Rust WASM (refinar una dirección con N iteraciones) |
| Hill-climb trigger | Botón "varita mágica" en overlay, sobre la posición actual |
| Overlay 3D | Modo drag-to-rotate sobre viewport (like Lychee), score badge en vivo |
| Second phase compute | No existe. El refinamiento es interactivo (drag manual + botón hill-climb) |
| Hill-climb top-K | No relevante — se ejecuta desde la posición actual, no sobre ranking |
| Hill-climb iterations | Pendiente definir (¿50?) |

## Arquitectura existente relevante

- `compute.ts`: scoring pipeline, consensus ranking, normalización
- `viewport.ts`: three.js render + yaw control actual (slider lineal + change event)
- `main.ts`: UI orchestration + worker pool
- `core/src/scoring.rs`: Rust reference — hill-climb se agregará aquí como nueva función WASM

## Open questions (para plan)

- N iteraciones hill-climb
- UI layout del overlay: badge de score + botón varita + indicador de modo activo
- Cómo integrar drag-to-rotate con orbit controls existentes (deshabilitar orbit mientras se arrastra el modelo?)
