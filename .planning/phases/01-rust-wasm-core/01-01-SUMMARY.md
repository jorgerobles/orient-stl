# 01-01-SUMMARY: WASM Build Toolchain

**Status:** ✅ Complete  
**Date:** 2026-07-11

## Deliverables

| Artifact | Status | Notes |
|---|---|---|
| `core/Cargo.toml` | ✅ | cdylib+rlib, wasm-bindgen 0.2, stl_io 0.11, serde 1, serde-wasm-bindgen 0.6, console_error_panic_hook 0.1 |
| `core/src/lib.rs` | ✅ | `#[wasm_bindgen(start)] init()`, `compute_orientations(bytes, config)` stub |
| `web/package.json` | ✅ | Vite 8.1, vite-plugin-wasm 3.6, TypeScript 5.7 |
| `web/vite.config.ts` | ✅ | wasm() plugin only (top-level-await unneeded with Vite 8) |
| `web/tsconfig.json` | ✅ | ESNext, bundler moduleResolution, strict |
| `web/index.html` | ✅ | Drop zone, file picker, config panel, results container |
| `web/src/main.ts` | ✅ | App shell init, WASM init pending status |

## Verification Results

| Check | Result |
|---|---|
| `cargo check` (native) | ✅ passes |
| `cargo check --target wasm32-unknown-unknown` | ✅ passes |
| `wasm-pack build core --target bundler --out-dir ../web/pkg` | ✅ orient_core_bg.wasm (19KB) |
| `npm install` | ✅ 20 packages, 0 vulnerabilities |
| `npx tsc --noEmit` | ✅ passes |
| `npx vite build` | ✅ builds to `dist/` |
| `npx vite` (dev server) | ✅ serves on localhost:5173 |

## Deviations from Plan

- **vite-plugin-top-level-await removed**: Incompatible with Vite 8.1 (rollup is ESM-only). Vite 8 handles WASM top-level imports natively.
- **stl_io crate name**: crates.io package is `stl_io` (underscore), not `stl-io` (hyphen).
- **console_error_panic_hook**: Moved to `[dependencies]` (not dev-dependencies) since lib.rs uses it in `#[wasm_bindgen(start)]`.
- **build:wasm script path**: Fixed from `../core` path (`web/` → project root).

## Commands

```bash
# Build WASM (from project root)
wasm-pack build core --target bundler --out-dir web/pkg

# Dev server
cd web && npm run dev
```
