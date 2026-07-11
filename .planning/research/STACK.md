# Technology Stack

**Project:** orient-stl
**Researched:** 2026-07-11

## Recommended Stack

### Core Framework
| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| Rust | 1.96+ (stable) | Computation core | Safe, fast, wasm32 target support, SIMD through LLVM |
| wasm-bindgen | 0.2.x (latest stable) | JS↔WASM bridge | Mature, generates .d.ts, handles memory management |
| wasm-pack | 0.13+ | Build orchestrator | Compiles, runs wasm-bindgen, runs wasm-opt, generates npm package |
| serde-wasm-bindgen | 0.6.x | Structured data serialization | Plain JS objects (not class handles) from Rust structs |
| stl-io | 0.11.x | STL binary parsing | Zero dependencies, takes impl Read, verified compiles to wasm32-unknown-unknown |

### UI Framework
| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| three.js | Latest stable | 3D viewport | De facto standard for browser WebGL. STLLoader, OrbitControls, quaternion support |
| Vite | 5+ | Build tool | Fast HMR, native WASM module support via plugin |
| TypeScript | 5+ | Type safety | Accurate types from wasm-bindgen .d.ts |
| vite-plugin-wasm | Latest | WASM module import | Enables `import` of .wasm files in Vite |
| vite-plugin-top-level-await | Latest | Async module init | Allows top-level await for WASM init() |

### Persistence & Export
| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| fflate | Latest | ZIP compression | Zero-dependency, browser-compatible, faster than JSZip |
| IndexedDB | Browser API | Favorites storage | Binary blob storage for thumbnails, structured clone for metadata |

### Dev Dependencies
| Technology | Version | Purpose | Why |
|------------|---------|---------|-----|
| wasm-bindgen-test | Latest | Rust WASM testing | Run tests in browser/Node.js WASM context |
| console_error_panic_hook | 0.1.x | Debug panic messages | Shows Rust panic messages in browser console (dev only) |

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| WASM build | wasm-pack | Manual cargo-build + wasm-bindgen CLI | wasm-pack handles wasm-opt, target flags, package generation. Manual is error-prone. |
| Data serialization | serde-wasm-bindgen (plain objects) | #[wasm_bindgen] structs (class handles) | Class handles require manual .free(), couple JS lifetime to WASM memory. Plain objects are independent. |
| STL parsing | stl-io | Vendored parser | stl-io is zero-dependency, well-tested, compiles to wasm32. Vendoring duplicates effort. |
| STL parsing (JS) | Rust stl-io | three.js STLLoader | STLLoader parses into three.js geometry objects. WASM needs raw triangles. Would require marshal from JS to WASM. |
| Convex hull | Vendored quickhull | hull crate (e.g., parry3d) | External crates pull in transitive deps (na, ndarray) that may not compile cleanly to wasm32. Vendored is ~300 lines and fully controlled. |
| Zip | fflate | JSZip | JSZip is larger, slower. fflate is modern, fast, zero-dependency. |
| Threading | None (main thread) | wasm-bindgen-rayon | Requires nightly Rust, SharedArrayBuffer, COOP/COEP headers, recompiled std. Not worth the complexity for 30-80ms compute time. |

## Installation

```bash
# Rust toolchain
rustup target add wasm32-unknown-unknown
cargo install wasm-pack

# Rust dependencies (Cargo.toml)
# [dependencies]
# wasm-bindgen = "0.2"
# serde = { version = "1", features = ["derive"] }
# serde-wasm-bindgen = "0.6"
# stl-io = "0.11"
# console_error_panic_hook = "0.1"  # dev only

# JS dependencies
npm install three @types/three fflate vite vite-plugin-wasm vite-plugin-top-level-await typescript
```

**Crate type configuration:**
```toml
# core/Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]   # cdylib for .wasm, rlib for cargo test
```

**Build command:**
```bash
wasm-pack build core --target bundler --out-dir ../web/pkg
# or with npm script:
"build:wasm": "wasm-pack build core --target bundler --out-dir ../web/pkg"
```

## Sources

- wasm-bindgen Guide — https://rustwasm.github.io/docs/wasm-bindgen/
- stl-io crate — https://crates.io/crates/stl-io
- serde-wasm-bindgen — https://crates.io/crates/serde-wasm-bindgen
- Vite WASM plugin — https://github.com/nicolo-ribaudo/vite-plugin-wasm
- fflate — https://github.com/101arrowz/fflate
- wasm-pack — https://rustwasm.github.io/wasm-pack/
