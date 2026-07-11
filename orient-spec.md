# Orient — Herramienta de auto-orientación para impresión en resina

Spec técnica para implementación. Objetivo: cargar un STL, calcular un ranking de orientaciones candidatas minimizando overhangs/soportes, navegar el ranking en un único viewport (next/prev vía quaternion), generar previews PNG, marcar favoritos y exportar STL(s) orientados.

## Stack

- **Core de cálculo**: Rust → WASM (`wasm-bindgen`), reutilizable fuera del navegador (CLI, Node, futura app de escritorio).
- **UI/render**: JS/TS + three.js (Vite).
- **Persistencia de favoritos**: IndexedDB (blobs binarios de thumbnails, no localStorage).
- **Export multi-archivo**: `fflate` para generar ZIP en cliente.

## Fundamento matemático

### 1. Score de overhang ponderado por área y altura

Para cada triángulo `i` con normal `n_i`, área `A_i`, y centroide `c_i`:

```
down_local = R⁻¹ · (0, 0, -1)          // vector "abajo" llevado al espacio local del modelo
cos_i      = dot(n_i, down_local)
height_i   = c_i.z_rotado - z_min_rotado   // altura del centroide sobre el punto de apoyo, en la orientación R

penalty_i  = A_i * max(0, cos_i - cos(θ_crítico)) * height_i
score(R)   = Σ penalty_i
```

`θ_crítico` configurable (30–35° típico en resina, medido desde la vertical).

**Optimización clave**: no rotar la malla. Precalcular `n_i`, `A_i`, `c_i` una sola vez al cargar; para cada candidato, transformar solo el vector `down` (y el `z_min` de referencia) al espacio local mediante `R⁻¹`. Cada evaluación de candidato es O(n_triángulos) en productos escalares, sin tocar geometría real.

### 2. Reducción de dimensionalidad: el espacio de búsqueda real es S², no SO(3)

El score depende únicamente de la dirección `down_local` (2 grados de libertad, dirección sobre la esfera), no de la rotación completa (3 grados de libertad — incluye yaw alrededor del eje vertical). El yaw es invariante para el score de overhang.

Consecuencias de diseño:
- Generación y refinamiento de candidatos trabaja en `S²` (θ, φ esféricos), no en cuaterniones completos — más barato y predecible (pattern search / Nelder-Mead en 2D).
- El yaw se fija **después**, con un criterio independiente por defecto: minimizar el bounding box en XY de la orientación elegida (mejor aprovechamiento de plataforma). Debe ser ajustable manualmente por el usuario en el viewport (slider) antes de exportar, para casos donde importa la estética del corte de soportes o la dirección de una pieza (ej. una espada).

### 3. Check de estabilidad (obligatorio, no opcional)

Sin esto el ranking puede proponer orientaciones que se caen físicamente:

- Calcular footprint de contacto: proyección XY de vértices a `z_min` (± epsilon) en la orientación candidata.
- Calcular centro de masa proyectado en XY (aproximable con centroide del convex hull si no se integra volumen real).
- Point-in-polygon: ¿el CoM proyectado cae dentro del hull 2D del footprint de contacto?
- Si no → candidato marcado `unstable`, penalización fuerte o exclusión del ranking según config.

### 4. Métricas multi-objetivo (no colapsar a un único número por defecto)

Cada candidato expone tres métricas independientes, más un composite score para el ranking por defecto:

```
overhangPenalty: number   // sección 1
estHeight: number          // altura total del modelo en esa orientación → proxy de tiempo de impresión
stability: "stable" | "unstable" | number (margen CoM-a-borde-de-footprint)
```

El usuario debe poder reordenar/filtrar por cualquiera de las tres, igual que PrusaSlicer separa "reduced overhang" de "lowest Z", en vez de forzar pesos arbitrarios en un único score.

## Generación de candidatos

Dos modos, configurables (no hardcodear uno):

- **`hull`**: normales de las caras del convex hull del modelo — buenos puntos de partida porque casi por definición minimizan overhang para modelos simples.
- **`hull_plus_sphere`**: hull + muestreo adicional tipo fibonacci-sphere (N direcciones configurable, ej. 80) para no depender solo del hull en modelos con detalle fino (texturas, minis) donde el hull pierde información.

Deduplicación: fusionar candidatos cuya dirección esté a menos de `dedupeAngleDeg` entre sí (evitar redundancia en el ranking).

Refinamiento local opcional (`refineIterations`): tras el ranking inicial, hill-climbing en `S²` sobre los top-K candidatos, perturbando `(θ, φ)` en pasos pequeños.

## API Rust (WASM)

```rust
#[wasm_bindgen]
pub fn compute_orientations(
    positions: &[f32],   // triángulos planos, espacio local del modelo
    config: JsValue,
) -> JsValue;             // Vec<Candidate>, ordenado por composite score
```

```ts
interface OrientConfig {
  mode: "hull" | "hull_plus_sphere";
  sphereSamples?: number;        // solo si hull_plus_sphere, ej. 80
  criticalAngleDeg: number;      // 30-35 típico en resina
  dedupeAngleDeg: number;
  refineIterations: number;      // 0 = sin hill-climbing
  excludeUnstable: boolean;
}

interface Candidate {
  id: string;
  quaternion: [number, number, number, number];  // incluye yaw ya resuelto por defecto
  overhangPenalty: number;
  estHeight: number;
  stability: "stable" | "unstable";
  stabilityMargin: number;
  contactArea: number;
  compositeScore: number;
}
```

La malla se pasa a WASM **una vez** (buffer de posiciones); normal/área/centroide por triángulo se precalculan dentro de Rust. Generación de candidatos, scoring, dedupe y refinamiento ocurren enteramente en Rust sin volver a cruzar la frontera JS↔WASM hasta devolver el `Vec<Candidate>` final — minimiza overhead de marshalling.

Convex hull: implementación pura Rust (quickhull incremental), evitar wrappers de qhull nativo por fricción de compilación a wasm.

## Estructura del repo

```
orient/
├── core/                     # Rust crate → wasm-bindgen
│   ├── src/
│   │   ├── lib.rs            # API pública (compute_orientations)
│   │   ├── mesh.rs           # normal/área/centroide por triángulo, precálculo único
│   │   ├── hull.rs           # convex hull puro Rust
│   │   ├── candidates.rs     # generación: hull | hull_plus_sphere, dedupe
│   │   ├── scoring.rs        # overhang penalty ponderado por área+altura (S²)
│   │   ├── stability.rs      # footprint, CoM proyectado, point-in-polygon
│   │   └── refine.rs         # hill-climbing en S² sobre top-K
│   └── Cargo.toml
├── web/                       # App JS (Vite + three.js)
│   ├── src/
│   │   ├── loadSTL.ts
│   │   ├── viewport.ts        # single viewport, next/prev vía mesh.quaternion
│   │   ├── thumbnails.ts      # render offscreen por candidato → PNG/blob
│   │   ├── favorites.ts       # IndexedDB: marcar/persistir candidatos favoritos
│   │   ├── exportSTL.ts       # bake de quaternion + export (single o ZIP multi)
│   │   └── main.ts
│   └── index.html
```

## Flujo de UI

1. Cargar STL → parseo en JS (three.js `STLLoader` o parser propio), buffer de triángulos plano.
2. Enviar buffer a WASM con `OrientConfig` → recibir `Candidate[]` ordenado por `compositeScore`.
3. Post-proceso JS: por cada candidato, render offscreen (`WebGLRenderer` en canvas oculto, cámara e iluminación fijas para comparabilidad) → thumbnail PNG. Una sola vez, no en cada navegación.
4. Viewport único: `mesh.quaternion.copy(candidates[i].quaternion)` en cada `next()/prev()` — sin recarga de geometría.
5. Slider de yaw manual disponible sobre la orientación activa (ajuste post-selección, no afecta score).
6. Marcar candidato(s) como favorito → persistir en IndexedDB (quaternion + thumbnail blob + métricas).
7. Exportar: si 1 favorito → STL baked único; si ≥2 → ZIP (`fflate`) con un STL por favorito, nombrado `modelo_orientNN_scoreX.stl`.

## Yaw: control circular con snap a geometría

El yaw (rotación alrededor del eje vertical, fijada después del score S²) se controla mediante un dial circular en el viewport — no un slider lineal 0–360°. El usuario arrastra un anillo alrededor del eje `down_local` para rotación libre continua.

### Snap candidates

Derivados de geometría que ya se computa (convex hull + rotating calipers), sin costo adicional significativo:

1. **Bbox local minima**: rotating calipers sobre el hull 2D proyectado en el plano perpendicular a `down_local` — enumera todos los mínimos locales de área de bounding box, no solo el global.
2. **Edge alignments**: cada arista del hull 2D corresponde a un ángulo donde una cara plana del modelo se alinea paralela a un eje de la plataforma (X o Y).

`yaw_snap_candidates(candidate) -> Vec<f32>` en Rust, computado una vez por candidato seleccionado.

### UI behavior

- **Arrastre libre** del anillo para control continuo.
- **Snap magnético**: si el ángulo de arrastre está a <3° (configurable) de un candidato, se ajusta con un tick visual.
- **Input numérico** junto al dial para valores exactos (ej. 90° para bisagra impresa in situ).
- **Botón "reset a auto"** que restaura el yaw por defecto (bbox mínimo).

Reusa rotating calipers que ya se necesitan para el yaw por defecto. No añade dependencias externas.

## Decisiones arquitectónicas

### Convex hull: vendido, no crate externo

`core/src/hull.rs` implementa quickhull incremental mínimo (~300 líneas, f32). Sin dependencias de crates.io.

Razones:
- WASM binary size: evitar arrastrar árbol transitivo de dependencias (ndarray + optional BLAS/rayon no compilan limpio a wasm32-unknown-unknown sin configuración extra).
- f32 en todo el pipeline, sin conversiones desde f64 genérico.
- Sin riesgo de feature flags que requieran `std::thread` en futuros bumps.

### STL parsing: en Rust, binario solamente

`core/src/stl.rs` — parsea STL binario directamente desde `&[u8]` (80-byte header, u32 triangle count, 50 bytes/triangle). Sin depender de `three.js STLLoader` para parsing.

`web/src/loadSTL.ts` se reduce a: `File → ArrayBuffer → WASM`, sin conocimiento del formato STL.

ASCII STL: diferido/opcional. Confirmar si los archivos reales son siempre binarios (lo habitual en herramientas adyacentes a slicers).

### stl-io crate

Preferir stl-io si compila limpio a wasm32-unknown-unknown. Hacer spike de 10 min para confirmar. Si no, vender el parser (~40 líneas).

## Roadmap de iteración

- **v1**: overhang penalty ponderado por área (sin altura), modo `hull` solamente, sin refinamiento, **con stability check binario** (rechazar orientaciones que se caen — ~40 líneas, mismo coste que el scoring). Objetivo: validar el pipeline WASM↔JS, ranking no-garbage, viewport next/prev + dial de yaw con snap.
- **v2**: añadir ponderación por altura, modo `hull_plus_sphere`, refinamiento S², `stabilityMargin` como campo ordenable continuo.
- **v3**: thumbnails (top-N configurable por score), favoritos + export multi-STL.
