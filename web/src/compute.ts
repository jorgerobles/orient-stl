export interface OriData {
  positions: Float32Array;
  normals: Float32Array;
  areas: Float32Array;
  directions: Float32Array;
}

export interface Candidate {
  id: string;
  quaternion: [number, number, number, number];
  overhangPenalty: number;
  footprint: number;
  maxCross: number;
  shadowed: number;
  surfaceQuality: number;
  estHeight: number;
  refinedOverhang: number;
  refineVariance: number;
  stability: 'stable' | 'unstable';
  stabilityMargin: number;
  contactArea: number;
  compositeScore: number;
}

export interface ComputeConfig {
  criticalAngleDeg: number;
  excludeUnstable: boolean;
  maxCandidates: number;
  refineIterations?: number;
}

export interface SliceResult {
  penalty: number;
  footprint: number;
  maxCross: number;
  shadowed: number;
  surface: number;
  quaternion: [number, number, number, number];
  height: number;
  refinedOverhang: number;
  refineVariance: number;
  stability: boolean;
  stabilityMargin: number;
  contactArea: number;
  idx: number;
  dir: [number, number, number];
}

/** Callback for injecting WASM batch refine into computeSlice. Returns K×4
 *  floats: each run's [dir_x, dir_y, dir_z, score]. Enables testing the
 *  refine pipeline without WASM. (D-08/D-09) */
export type RefineFn = (
  dir: [number, number, number],
  positions: Float32Array,
  normals: Float32Array,
  areas: Float32Array,
  criticalAngleDeg: number,
) => number[] | Float32Array;

export function directionFromIndex(data: OriData, i: number): [number, number, number] {
  const di = i * 3;
  return [data.directions[di], data.directions[di + 1], data.directions[di + 2]];
}

function cross(a: [number, number, number], b: [number, number, number]): [number, number, number] {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

function perpendicularBasis(d: [number, number, number]): [[number, number, number], [number, number, number]] {
  const a: [number, number, number] = Math.abs(d[0]) < 0.9 ? [1, 0, 0] : [0, 1, 0];
  let e1 = cross(d, a);
  const l1 = Math.sqrt(e1[0] * e1[0] + e1[1] * e1[1] + e1[2] * e1[2]);
  if (l1 < 1e-12) return [[1, 0, 0], [0, 1, 0]];
  e1 = [e1[0] / l1, e1[1] / l1, e1[2] / l1];
  const e2 = cross(d, e1);
  return [e1, e2];
}

function convexHull2D(points: [number, number][]): [number, number][] {
  if (points.length < 3) return [...points];
  const pts = points.map(p => ({ x: p[0], y: p[1] }));
  pts.sort((a, b) => a.x - b.x || a.y - b.y);
  const cross2 = (o: { x: number; y: number }, a: { x: number; y: number }, b: { x: number; y: number }) =>
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
  const lower: typeof pts = [];
  for (const p of pts) {
    while (lower.length >= 2 && cross2(lower[lower.length - 2], lower[lower.length - 1], p) <= 0) lower.pop();
    lower.push(p);
  }
  const upper: typeof pts = [];
  for (let i = pts.length - 1; i >= 0; i--) {
    const p = pts[i];
    while (upper.length >= 2 && cross2(upper[upper.length - 2], upper[upper.length - 1], p) <= 0) upper.pop();
    upper.push(p);
  }
  lower.pop(); upper.pop();
  return [...lower, ...upper].map(p => [p.x, p.y] as [number, number]);
}

function polygonArea(poly: [number, number][]): number {
  if (poly.length < 3) return 0;
  let area = 0;
  for (let i = 0; i < poly.length; i++) {
    const j = (i + 1) % poly.length;
    area += poly[i][0] * poly[j][1];
    area -= poly[j][0] * poly[i][1];
  }
  return Math.abs(area) * 0.5;
}

function pointInConvexPolygon(p: [number, number], poly: [number, number][]): boolean {
  if (poly.length < 3) return true;
  let sign = 0;
  for (let i = 0; i < poly.length; i++) {
    const j = (i + 1) % poly.length;
    const crossVal = (poly[j][0] - poly[i][0]) * (p[1] - poly[i][1]) - (poly[j][1] - poly[i][1]) * (p[0] - poly[i][0]);
    if (Math.abs(crossVal) < 1e-6) continue;
    const s = Math.sign(crossVal);
    if (sign === 0) sign = s;
    else if (Math.abs(s - sign) > 0.1) return false;
  }
  return true;
}

function minEdgeDistance(p: [number, number], poly: [number, number][]): number {
  if (poly.length < 3) return Infinity;
  let minD = Infinity;
  for (let i = 0; i < poly.length; i++) {
    const j = (i + 1) % poly.length;
    const dx = poly[j][0] - poly[i][0];
    const dy = poly[j][1] - poly[i][1];
    const lenSq = dx * dx + dy * dy;
    if (lenSq < 1e-12) continue;
    let t = ((p[0] - poly[i][0]) * dx + (p[1] - poly[i][1]) * dy) / lenSq;
    t = Math.max(0, Math.min(1, t));
    const px = poly[i][0] + t * dx;
    const py = poly[i][1] + t * dy;
    const d = Math.sqrt((p[0] - px) ** 2 + (p[1] - py) ** 2);
    if (d < minD) minD = d;
  }
  return minD;
}

function angleBetween(a: [number, number, number], b: [number, number, number]): number {
  const dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  return Math.acos(Math.max(-1, Math.min(1, dot))) * 180 / Math.PI;
}

export function scoreCandidate(
  dir: [number, number, number],
  normals: Float32Array,
  areas: Float32Array,
  criticalAngleDeg: number,
): number {
  const theta = criticalAngleDeg * Math.PI / 180;
  const cosCrit = Math.cos(theta);
  let penalty = 0;
  const triCount = normals.length / 3;
  for (let i = 0; i < triCount; i++) {
    const ni = i * 3;
    const cos_i = dir[0] * normals[ni] + dir[1] * normals[ni + 1] + dir[2] * normals[ni + 2];
    if (cos_i > cosCrit) {
      penalty += areas[i] * (cos_i - cosCrit);
    }
  }
  return isFinite(penalty) ? penalty : 0;
}

export function checkStability(
  dir: [number, number, number],
  verts: Float32Array,
): { stable: boolean; margin: number; contactArea: number } {
  const n = verts.length / 3;
  const dnLen = Math.sqrt(dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]);
  if (dnLen < 1e-8) return { stable: true, margin: 1, contactArea: 0 };
  const dn: [number, number, number] = [dir[0] / dnLen, dir[1] / dnLen, dir[2] / dnLen];
  const up: [number, number, number] = [-dn[0], -dn[1], -dn[2]];
  let upX: [number, number, number], upY: [number, number, number];
  if (Math.abs(up[0]) < 0.9) upX = cross(up, [1, 0, 0]);
  else upX = cross(up, [0, 1, 0]);
  const uxLen = Math.sqrt(upX[0] * upX[0] + upX[1] * upX[1] + upX[2] * upX[2]);
  if (uxLen > 1e-8) upX = [upX[0] / uxLen, upX[1] / uxLen, upX[2] / uxLen];
  upY = cross(up, upX);

  let minDot = Infinity, maxDot = -Infinity;
  for (let i = 0; i < n; i++) {
    const vi = i * 3;
    const d = verts[vi] * dn[0] + verts[vi + 1] * dn[1] + verts[vi + 2] * dn[2];
    if (d < minDot) minDot = d;
    if (d > maxDot) maxDot = d;
  }

  const eps = 1e-4;
  const footprint: [number, number][] = [];
  for (let i = 0; i < n; i++) {
    const vi = i * 3;
    const d = verts[vi] * dn[0] + verts[vi + 1] * dn[1] + verts[vi + 2] * dn[2];
    if (Math.abs(d - minDot) < eps) {
      footprint.push([
        verts[vi] * upX[0] + verts[vi + 1] * upX[1] + verts[vi + 2] * upX[2],
        verts[vi] * upY[0] + verts[vi + 1] * upY[1] + verts[vi + 2] * upY[2],
      ]);
    }
  }
  if (footprint.length < 3) return { stable: true, margin: 1, contactArea: 0 };

  const hull = convexHull2D(footprint);
  const contactArea = polygonArea(hull);

  let comX = 0, comY = 0, comZ = 0;
  for (let i = 0; i < n; i++) { const vi = i * 3; comX += verts[vi]; comY += verts[vi + 1]; comZ += verts[vi + 2]; }
  comX /= n; comY /= n; comZ /= n;
  const comPx = comX * upX[0] + comY * upX[1] + comZ * upX[2];
  const comPy = comX * upY[0] + comY * upY[1] + comZ * upY[2];

  if (!pointInConvexPolygon([comPx, comPy], hull)) return { stable: false, margin: 0, contactArea };
  const margin = minEdgeDistance([comPx, comPy], hull);
  return { stable: true, margin: contactArea > 1e-8 ? margin / Math.sqrt(contactArea) : 1, contactArea };
}

export function computeDefaultYaw(dir: [number, number, number], verts: Float32Array): [number, number, number, number] {
  const dnLen = Math.sqrt(dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]);
  if (dnLen < 1e-8) return [1, 0, 0, 0];
  const dn: [number, number, number] = [dir[0] / dnLen, dir[1] / dnLen, dir[2] / dnLen];
  const up: [number, number, number] = [-dn[0], -dn[1], -dn[2]];
  let upX: [number, number, number], upY: [number, number, number];
  if (Math.abs(up[0]) < 0.9) upX = cross(up, [1, 0, 0]);
  else upX = cross(up, [0, 1, 0]);
  const uxLen = Math.sqrt(upX[0] * upX[0] + upX[1] * upX[1] + upX[2] * upX[2]);
  if (uxLen > 1e-8) upX = [upX[0] / uxLen, upX[1] / uxLen, upX[2] / uxLen];
  upY = cross(up, upX);

  const n = verts.length / 3;
  const pts2D: [number, number][] = [];
  for (let i = 0; i < n; i++) {
    const vi = i * 3;
    pts2D.push([verts[vi] * upX[0] + verts[vi + 1] * upX[1] + verts[vi + 2] * upX[2],
                verts[vi] * upY[0] + verts[vi + 1] * upY[1] + verts[vi + 2] * upY[2]]);
  }
  const hull = convexHull2D(pts2D);
  let bestAngle = 0, bestArea = Infinity;
  for (let s = 0; s < 180; s++) {
    const angle = (s / 180) * Math.PI;
    const ca = Math.cos(angle), sa = Math.sin(angle);
    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    for (const [px, py] of hull) {
      const rx = px * ca - py * sa, ry = px * sa + py * ca;
      if (rx < minX) minX = rx; if (rx > maxX) maxX = rx;
      if (ry < minY) minY = ry; if (ry > maxY) maxY = ry;
    }
    const area = (maxX - minX) * (maxY - minY);
    if (area < bestArea) { bestArea = area; bestAngle = angle; }
  }
  const half = bestAngle / 2;
  return [Math.cos(half), dn[0] * Math.sin(half), dn[1] * Math.sin(half), dn[2] * Math.sin(half)];
}

export function computeHeight(dir: [number, number, number], verts: Float32Array): number {
  let minH = Infinity, maxH = -Infinity;
  for (let i = 0; i < verts.length; i += 3) {
    const d = verts[i] * dir[0] + verts[i + 1] * dir[1] + verts[i + 2] * dir[2];
    if (d < minH) minH = d;
    if (d > maxH) maxH = d;
  }
  return Math.abs(maxH - minH);
}

/**
 * Decimate mesh data to ~targetElements for faster approximate computation.
 * Samples by TRIANGLE so positions stay synced with normals/areas (each kept
 * triangle contributes its 3 vertices). Required for maxCrossSection which
 * needs per-triangle centroids.
 */
export function decimateForScore(data: OriData, targetElements: number): OriData {
  const triCount = data.normals.length / 3;
  if (triCount <= targetElements) return data;

  const triStep = Math.max(1, Math.floor(triCount / targetElements));
  const newTriCount = Math.ceil(triCount / triStep);
  const newNormals = new Float32Array(newTriCount * 3);
  const newAreas = new Float32Array(newTriCount);
  // Per-triangle vertices: 3 verts × 3 coords = 9 entries per triangle.
  // WASM returns positions as per-triangle vertex groups (triCount × 3 verts).
  const newPositions = new Float32Array(newTriCount * 9);
  for (let i = 0; i < newTriCount; i++) {
    const srcTri = i * triStep;
    const srcN = srcTri * 3;
    const dstN = i * 3;
    newNormals[dstN] = data.normals[srcN];
    newNormals[dstN + 1] = data.normals[srcN + 1];
    newNormals[dstN + 2] = data.normals[srcN + 2];
    newAreas[i] = data.areas[srcTri];
    // Copy the 3 vertices (9 coords) of this triangle.
    const srcV = srcTri * 9;
    const dstV = i * 9;
    newPositions[dstV] = data.positions[srcV];
    newPositions[dstV + 1] = data.positions[srcV + 1];
    newPositions[dstV + 2] = data.positions[srcV + 2];
    newPositions[dstV + 3] = data.positions[srcV + 3];
    newPositions[dstV + 4] = data.positions[srcV + 4];
    newPositions[dstV + 5] = data.positions[srcV + 5];
    newPositions[dstV + 6] = data.positions[srcV + 6];
    newPositions[dstV + 7] = data.positions[srcV + 7];
    newPositions[dstV + 8] = data.positions[srcV + 8];
  }

  return { positions: newPositions, normals: newNormals, areas: newAreas, directions: data.directions };
}

/** H4 — footprint (shadow) area. Sum of each triangle's projected area onto the
 *  plane whose normal is `dir`. O(N), one dot + abs + mul per triangle. */
export function footprintArea(
  dir: [number, number, number],
  normals: Float32Array,
  areas: Float32Array,
): number {
  const triCount = normals.length / 3;
  let total = 0;
  for (let i = 0; i < triCount; i++) {
    const ni = i * 3;
    const cosI = Math.abs(dir[0] * normals[ni] + dir[1] * normals[ni + 1] + dir[2] * normals[ni + 2]);
    total += areas[i] * cosI;
  }
  return isFinite(total) ? total : 0;
}

/** H5 — surface-quality (axis-misalignment) score. Port of PrusaSlicer's
 *  get_misalginment_score (Rotfinder.cpp:88). For each face, sums the L1 norm
 *  of the normal expressed in the orientation frame (dn, e1, e2):
 *    area × (|n·dn| + |n·e1| + |n·e2|)
 *  HIGHER = better. The L1 norm is minimised (=1) when a face aligns with a
 *  single frame axis (a big flat shelf/wall) and maximised (=√3) when the face
 *  is diagonal to all three. PrusaSlicer maximises this to discourage large
 *  axis-aligned shelves/walls that print poorly. O(N). */
export function misalignmentScore(
  dir: [number, number, number],
  normals: Float32Array,
  areas: Float32Array,
): number {
  const dnLen = Math.sqrt(dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]);
  if (dnLen < 1e-12) return 0;
  const dn: [number, number, number] = [dir[0] / dnLen, dir[1] / dnLen, dir[2] / dnLen];
  const [e1, e2] = perpendicularBasis(dn);
  const triCount = normals.length / 3;
  let total = 0;
  for (let i = 0; i < triCount; i++) {
    const ni = i * 3;
    const nx = normals[ni], ny = normals[ni + 1], nz = normals[ni + 2];
    const align =
      Math.abs(nx * dn[0] + ny * dn[1] + nz * dn[2]) +
      Math.abs(nx * e1[0] + ny * e1[1] + nz * e1[2]) +
      Math.abs(nx * e2[0] + ny * e2[1] + nz * e2[2]);
    total += areas[i] * align;
  }
  return isFinite(total) ? total : 0;
}

/** H2 — max cross-section area (Z-histogram approximation). Bins each triangle
 *  by its centroid projected onto `dir` into `bins` slices, sums projected area
 *  per bin, returns the max bin. Proxy for peel force. O(N).
 *  Requires positions as per-triangle vertex groups (9 entries/triangle). */
export function maxCrossSection(
  dir: [number, number, number],
  positions: Float32Array,
  normals: Float32Array,
  areas: Float32Array,
  bins: number,
): number {
  const triCount = normals.length / 3;
  if (triCount === 0 || bins === 0) return 0;
  let lo = Infinity, hi = -Infinity;
  const centroidsD = new Float32Array(triCount);
  for (let i = 0; i < triCount; i++) {
    const vi = i * 9;
    const cd = (dir[0] * (positions[vi] + positions[vi + 3] + positions[vi + 6])
      + dir[1] * (positions[vi + 1] + positions[vi + 4] + positions[vi + 7])
      + dir[2] * (positions[vi + 2] + positions[vi + 5] + positions[vi + 8])) / 3;
    centroidsD[i] = cd;
    if (cd < lo) lo = cd;
    if (cd > hi) hi = cd;
  }
  const span = Math.max(hi - lo, 1e-9);
  const scale = bins / span;
  const hist = new Float32Array(bins);
  for (let i = 0; i < triCount; i++) {
    let b = Math.floor((centroidsD[i] - lo) * scale);
    if (b >= bins) b = bins - 1;
    if (b < 0) b = 0;
    const ni = i * 3;
    const cosI = Math.abs(dir[0] * normals[ni] + dir[1] * normals[ni + 1] + dir[2] * normals[ni + 2]);
    hist[b] += areas[i] * cosI;
  }
  let best = 0;
  for (let i = 0; i < bins; i++) if (hist[i] > best) best = hist[i];
  return isFinite(best) ? best : 0;
}

/** H11 — shadowed-overhang fraction. Port of Rust core/src/scoring.rs.
 *  2.5D height-field: rasterises each triangle into a grid perpendicular to
 *  `dir`, records min centroid-height per cell, then queries overhang triangles
 *  for whether their centroid sits above an existing surface (shadowed).
 *  Returns fraction of overhang area that is shadowed, in [0,1]. */
export function shadowedOverhangFraction(
  dir: [number, number, number],
  positions: Float32Array,
  normals: Float32Array,
  areas: Float32Array,
  criticalAngleDeg: number,
  gridRes: number = 32,
  tolFrac: number = 0.02,
  basis?: [[number, number, number], [number, number, number]],
): number {
  const tri = normals.length / 3;
  if (tri === 0 || gridRes === 0) return 0;
  const dnLen = Math.sqrt(dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]);
  if (dnLen < 1e-12) return 0;
  const dn: [number, number, number] = [dir[0] / dnLen, dir[1] / dnLen, dir[2] / dnLen];
  const [e1, e2] = basis ?? perpendicularBasis(dn);

  const u = new Float32Array(tri);
  const v = new Float32Array(tri);
  const h = new Float32Array(tri);
  let uMin = Infinity, uMax = -Infinity;
  let vMin = Infinity, vMax = -Infinity;
  let hMin = Infinity, hMax = -Infinity;
  for (let i = 0; i < tri; i++) {
    const vi = i * 9;
    const cx = (positions[vi] + positions[vi + 3] + positions[vi + 6]) / 3;
    const cy = (positions[vi + 1] + positions[vi + 4] + positions[vi + 7]) / 3;
    const cz = (positions[vi + 2] + positions[vi + 5] + positions[vi + 8]) / 3;
    const uu = cx * e1[0] + cy * e1[1] + cz * e1[2];
    const vv = cx * e2[0] + cy * e2[1] + cz * e2[2];
    const hh = cx * dn[0] + cy * dn[1] + cz * dn[2];
    u[i] = uu; v[i] = vv; h[i] = hh;
    if (uu < uMin) uMin = uu;
    if (uu > uMax) uMax = uu;
    if (vv < vMin) vMin = vv;
    if (vv > vMax) vMax = vv;
    if (hh < hMin) hMin = hh;
    if (hh > hMax) hMax = hh;
  }
  const uSpan = Math.max(uMax - uMin, 1e-9);
  const vSpan = Math.max(vMax - vMin, 1e-9);
  const hSpan = Math.max(hMax - hMin, 1e-9);
  const tol = Math.max(tolFrac * hSpan, 1e-9);
  const uScale = gridRes / uSpan;
  const vScale = gridRes / vSpan;
  const invUScale = 1 / uScale;
  const invVScale = 1 / vScale;

  // Height field: min h per cell, f32::INFINITY = empty
  const field = new Float32Array(gridRes * gridRes);
  field.fill(Infinity);
  for (let i = 0; i < tri; i++) {
    const vi = i * 9;
    const u0 = positions[vi] * e1[0] + positions[vi + 1] * e1[1] + positions[vi + 2] * e1[2];
    const v0 = positions[vi] * e2[0] + positions[vi + 1] * e2[1] + positions[vi + 2] * e2[2];
    const u1 = positions[vi + 3] * e1[0] + positions[vi + 4] * e1[1] + positions[vi + 5] * e1[2];
    const v1 = positions[vi + 3] * e2[0] + positions[vi + 4] * e2[1] + positions[vi + 5] * e2[2];
    const u2 = positions[vi + 6] * e1[0] + positions[vi + 7] * e1[1] + positions[vi + 8] * e1[2];
    const v2 = positions[vi + 6] * e2[0] + positions[vi + 7] * e2[1] + positions[vi + 8] * e2[2];
    const cuMin = Math.max(0, Math.floor((Math.min(u0, u1, u2) - uMin) * uScale));
    const cuMax = Math.min(gridRes - 1, Math.floor((Math.max(u0, u1, u2) - uMin) * uScale));
    const cvMin = Math.max(0, Math.floor((Math.min(v0, v1, v2) - vMin) * vScale));
    const cvMax = Math.min(gridRes - 1, Math.floor((Math.max(v0, v1, v2) - vMin) * vScale));
    if (cuMin > cuMax || cvMin > cvMax) continue;

    // Edge functions for barycentric containment
    const w0u = v1 - v2, w0v = u2 - u1;
    const w1u = v2 - v0, w1v = u0 - u2;
    const w2u = v0 - v1, w2v = u1 - u0;
    const area = u0 * w0u + u1 * w1u + u2 * w2u;
    if (Math.abs(area) < 1e-12) continue;
    const invArea = 1 / area;

    for (let cv = cvMin; cv <= cvMax; cv++) {
      const vc = vMin + (cv + 0.5) * invVScale;
      const rowOffset = cv * gridRes;
      for (let cu = cuMin; cu <= cuMax; cu++) {
        const uc = uMin + (cu + 0.5) * invUScale;
        const b0 = (w0u * (uc - u2) + w0v * (vc - v2)) * invArea;
        const b1 = (w1u * (uc - u2) + w1v * (vc - v2)) * invArea;
        const b2 = 1 - b0 - b1;
        if (b0 >= -0.02 && b1 >= -0.02 && b2 >= -0.02) {
          const cell = rowOffset + cu;
          if (h[i] < field[cell]) field[cell] = h[i];
        }
      }
    }
  }

  // Query overhang triangles
  const cosCrit = Math.cos(criticalAngleDeg * Math.PI / 180);
  let overArea = 0, shadowArea = 0;
  for (let i = 0; i < tri; i++) {
    const ni = i * 3;
    const cosI = dn[0] * normals[ni] + dn[1] * normals[ni + 1] + dn[2] * normals[ni + 2];
    if (cosI <= cosCrit) continue;
    overArea += areas[i];
    let cu = Math.floor((u[i] - uMin) * uScale);
    if (cu >= gridRes) cu = gridRes - 1;
    if (cu < 0) cu = 0;
    let cv = Math.floor((v[i] - vMin) * vScale);
    if (cv >= gridRes) cv = gridRes - 1;
    if (cv < 0) cv = 0;
    const floor = field[cv * gridRes + cu];
    if (isFinite(floor) && h[i] - floor > tol) {
      shadowArea += areas[i];
    }
  }
  if (overArea <= 0) return 0;
  const frac = shadowArea / overArea;
  return isFinite(frac) ? Math.max(0, Math.min(1, frac)) : 0;
}

/** Compute shadowed-overhang fraction minimised over 8 yaw samples (0°–315°).
 *  Yaw rotates the projection basis around `dir`, which is equivalent to
 *  rotating the model laterally. Returns the minimum shadow fraction found. */
export function minShadowedOverhang(
  dir: [number, number, number],
  positions: Float32Array,
  normals: Float32Array,
  areas: Float32Array,
  criticalAngleDeg: number,
  gridRes: number = 32,
  tolFrac: number = 0.02,
  yawSamples: number = 8,
): number {
  const dnLen = Math.sqrt(dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]);
  if (dnLen < 1e-12) return 0;
  const dn: [number, number, number] = [dir[0] / dnLen, dir[1] / dnLen, dir[2] / dnLen];
  const [e1, e2] = perpendicularBasis(dn);
  let best = shadowedOverhangFraction(dir, positions, normals, areas, criticalAngleDeg, gridRes, tolFrac, [e1, e2]);
  if (yawSamples <= 1) return best;
  for (let s = 1; s < yawSamples; s++) {
    const theta = (s / yawSamples) * Math.PI * 2;
    const c = Math.cos(theta), sc = Math.sin(theta);
    const r1: [number, number, number] = [e1[0] * c + e2[0] * sc, e1[1] * c + e2[1] * sc, e1[2] * c + e2[2] * sc];
    const r2: [number, number, number] = [-e1[0] * sc + e2[0] * c, -e1[1] * sc + e2[1] * c, -e1[2] * sc + e2[2] * c];
    const shadow = shadowedOverhangFraction(dir, positions, normals, areas, criticalAngleDeg, gridRes, tolFrac, [r1, r2]);
    if (shadow < best) best = shadow;
  }
  return best;
}

/** Quaternion that rotates vector `a` to align with vector `b` (both normalized). */
function quaternionAlign(a: [number, number, number], b: [number, number, number]): [number, number, number, number] {
  const dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  if (dot > 0.9999) return [1, 0, 0, 0];
  if (dot < -0.9999) {
    // 180° around a perpendicular axis
    let axis: [number, number, number];
    if (Math.abs(a[0]) < 0.9) axis = cross(a, [1, 0, 0]);
    else axis = cross(a, [0, 1, 0]);
    const al = Math.sqrt(axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]);
    axis = [axis[0] / al, axis[1] / al, axis[2] / al];
    return [0, axis[0], axis[1], axis[2]];
  }
  const axis = cross(a, b);
  const al = Math.sqrt(axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]);
  const naxis: [number, number, number] = [axis[0] / al, axis[1] / al, axis[2] / al];
  const half = Math.acos(dot) / 2;
  const s = Math.sin(half);
  return [Math.cos(half), naxis[0] * s, naxis[1] * s, naxis[2] * s];
}

function multiplyQuats(
  a: [number, number, number, number],
  b: [number, number, number, number],
): [number, number, number, number] {
  return [
    a[0] * b[0] - a[1] * b[1] - a[2] * b[2] - a[3] * b[3],
    a[0] * b[1] + a[1] * b[0] + a[2] * b[3] - a[3] * b[2],
    a[0] * b[2] - a[1] * b[3] + a[2] * b[0] + a[3] * b[1],
    a[0] * b[3] + a[1] * b[2] - a[2] * b[1] + a[3] * b[0],
  ];
}

/** Score a contiguous range of directions. Each worker calls this on its chunk.
 *  Returns a FULL quaternion that aligns the candidate direction to -Y (build plate)
 *  and then applies the optimal yaw. */
export function computeSlice(
  data: OriData,
  config: ComputeConfig,
  dirStart: number,
  dirCount: number,
  onProgress?: (pct: number) => void,
  refineFn?: RefineFn,
): SliceResult[] {
  const results: SliceResult[] = [];
  for (let i = 0; i < dirCount; i++) {
    const dir = directionFromIndex(data, dirStart + i);
    const penalty = scoreCandidate(dir, data.normals, data.areas, config.criticalAngleDeg);
    const foot = footprintArea(dir, data.normals, data.areas);
    const mcross = maxCrossSection(dir, data.positions, data.normals, data.areas, 64);
    const shad = minShadowedOverhang(dir, data.positions, data.normals, data.areas, config.criticalAngleDeg);
    const surf = misalignmentScore(dir, data.normals, data.areas);
    const stab = checkStability(dir, data.positions);
    const height = computeHeight(dir, data.positions);
    const qYaw = computeDefaultYaw(dir, data.positions);
    const qAlign = quaternionAlign(dir, [0, -1, 0]);
    const qFull = multiplyQuats(qYaw, qAlign);

    // Batch refine (D-08/D-09): if refineFn is provided, call it to get K×4
    // floats, extract K scores, compute refinedOverhang (min) and
    // refineVariance (population stddev). Guard with try/catch so one bad
    // direction doesn't kill all computation (T-03.5-03).
    let refinedOverhang = penalty;
    let refineVariance = 0;
    if (refineFn) {
      try {
        const refineOut = refineFn(dir, data.positions, data.normals, data.areas, config.criticalAngleDeg);
        const k = refineOut.length / 4;
        if (k > 0) {
          const scores: number[] = [];
          for (let j = 0; j < k; j++) scores.push(refineOut[j * 4 + 3]);
          refinedOverhang = Math.min(...scores);
          const mean = scores.reduce((a, b) => a + b, 0) / k;
          const variance = scores.reduce((a, b) => a + (b - mean) * (b - mean), 0) / k;
          refineVariance = Math.sqrt(variance);
        }
      } catch (err) {
        console.warn('refineFn failed for direction, using raw penalty:', err);
        refinedOverhang = penalty;
        refineVariance = 0;
      }
    }

    results.push({
      dir,
      penalty,
      footprint: foot,
      maxCross: mcross,
      shadowed: shad,
      surface: surf,
      quaternion: qFull,
      height,
      refinedOverhang,
      refineVariance,
      stability: stab.stable,
      stabilityMargin: stab.margin,
      contactArea: stab.contactArea,
      idx: dirStart + i,
    });
    if (onProgress && i % 5 === 0) onProgress(Math.round((i / dirCount) * 100));
  }
  return results;
}

/** Merge slice results from all workers into final diverse candidates.
 *  Selection uses angle-diversity on the sorted list; each candidate
 *  carries raw overhang/footprint/maxCross for later re-ranking.
 *  `weights` controls weighted-sum sorting (for profile-aware selection).
 *  `ranker` selects the sort formula before diversity picking:
 *    "consensus" — 1 − max(normalised costs), "weights" — weighted sum,
 *    "topsis" — falls back to consensus for the selection pass. */
export function mergeCandidates(
  slices: SliceResult[][],
  config: ComputeConfig,
  weights?: ScoreWeights,
  ranker?: string,
): Candidate[] {
  const all = slices.flat();
  if (weights && (!ranker || ranker === 'weights')) {
    const wActive = weights.wOverhang + weights.wFootprint + weights.wCross + weights.wSurface + weights.wHeight;
    if (wActive > 0) {
      const oVals = all.map(s => s.refinedOverhang);
      const fVals = all.map(s => s.footprint);
      const cVals = all.map(s => s.maxCross);
      const sVals = all.map(s => s.surface);
      const hVals = all.map(s => s.height);
      const oL = Math.min(...oVals), oH = Math.max(...oVals);
      const fL = Math.min(...fVals), fH = Math.max(...fVals);
      const cL = Math.min(...cVals), cH = Math.max(...cVals);
      const sL = Math.min(...sVals), sH = Math.max(...sVals);
      const hL = Math.min(...hVals), hH = Math.max(...hVals);
      const oS = Math.max(oH - oL, 1e-9), fS = Math.max(fH - fL, 1e-9);
      const cS = Math.max(cH - cL, 1e-9), sS = Math.max(sH - sL, 1e-9);
      const hS = Math.max(hH - hL, 1e-9);
      all.sort((a, b) => {
        const aScore =
          weights!.wOverhang * ((a.refinedOverhang - oL) / oS) +
          weights!.wFootprint   * ((a.footprint - fL) / fS) +
          weights!.wCross       * ((a.maxCross - cL) / cS) +
          weights!.wSurface     * ((a.surface - sL) / sS) +
          weights!.wHeight      * ((a.height - hL) / hS);
        const bScore =
          weights!.wOverhang * ((b.refinedOverhang - oL) / oS) +
          weights!.wFootprint   * ((b.footprint - fL) / fS) +
          weights!.wCross       * ((b.maxCross - cL) / cS) +
          weights!.wSurface     * ((b.surface - sL) / sS) +
          weights!.wHeight      * ((b.height - hL) / hS);
        return aScore - bScore;
      });
    } else {
      all.sort((a, b) => a.refinedOverhang - b.refinedOverhang);
    }
  } else if (ranker === 'consensus' || ranker === 'topsis') {
    // Consensus (or TOPSIS proxy): 1 − max(normalised costs).
    const oVals = all.map(s => s.refinedOverhang);
    const fVals = all.map(s => s.footprint);
    const cVals = all.map(s => s.maxCross);
    const sVals = all.map(s => s.surface);
    const hVals = all.map(s => s.height);
    const oL = Math.min(...oVals), oH = Math.max(...oVals);
    const fL = Math.min(...fVals), fH = Math.max(...fVals);
    const cL = Math.min(...cVals), cH = Math.max(...cVals);
    const sL = Math.min(...sVals), sH = Math.max(...sVals);
    const hL = Math.min(...hVals), hH = Math.max(...hVals);
    const oS = Math.max(oH - oL, 1e-9), fS = Math.max(fH - fL, 1e-9);
    const cS = Math.max(cH - cL, 1e-9), sS = Math.max(sH - sL, 1e-9);
    const hS = Math.max(hH - hL, 1e-9);
    // Schwartzian transform: decorate, sort, undecorate.
    const decorated = all.map((s, i) => ({
      idx: i,
      score: 1 - Math.max(
        (s.refinedOverhang - oL) / oS,
        (s.footprint - fL) / fS,
        (s.maxCross - cL) / cS,
        (s.surface - sL) / sS,
        (s.height - hL) / hS,
      ),
    }));
    decorated.sort((a, b) => b.score - a.score);
    const sorted = decorated.map(d => all[d.idx]);
    all.length = 0; all.push(...sorted);
  } else {
    all.sort((a, b) => a.refinedOverhang - b.refinedOverhang);
  }
  const results: Candidate[] = [];
  const picked: [number, number, number][] = [];
  const minAngle = 15;
  for (const item of all) {
    if (!config.excludeUnstable || item.stability) {
      const tooClose = picked.some(p => angleBetween(item.dir, p) < minAngle);
      if (!tooClose) {
        results.push({
          id: `candidate-${item.idx}`,
          quaternion: item.quaternion,
          overhangPenalty: item.penalty,
          footprint: item.footprint,
          maxCross: item.maxCross,
          shadowed: item.shadowed,
          surfaceQuality: item.surface,
          estHeight: item.height,
          refinedOverhang: item.refinedOverhang,
          refineVariance: item.refineVariance,
          stability: item.stability ? 'stable' : 'unstable',
          stabilityMargin: item.stabilityMargin,
          contactArea: item.contactArea,
          compositeScore: item.penalty, // initial; rankByWeights overrides
        });
        picked.push(item.dir);
        if (results.length >= config.maxCandidates) break;
      }
    }
  }
  return results;
}

/** Weight configuration for the composite score. Each weight scales a
 *  min-max-normalised component (cost form, lower = better). `wSurface` and
 *  `wHeight` are included so every heuristic can be tuned per use case. */
export interface ScoreWeights {
  wOverhang: number;
  wFootprint: number;
  wCross: number;
  wSurface: number;
  wHeight: number;
}

import { loadProfiles } from './profiles';
export const WEIGHT_PRESETS: Record<string, ScoreWeights> = loadProfiles();

/** Re-rank candidates by a weighted composite (min-max normalised per component).
 *  Minimised metrics (overhang/footprint/cross/height) normalise directly;
 *  maximised metrics (surfaceQuality) are inverted so that lower composite =
 *  better. Returns a NEW sorted array; does not mutate input. Pure — safe to
 *  call on weight-config changes without recomputing slices. */
export function rankByWeights(candidates: Candidate[], weights: ScoreWeights): Candidate[] {
  if (candidates.length === 0) return [];
  let oMin = Infinity, oMax = -Infinity;
  let fMin = Infinity, fMax = -Infinity;
  let cMin = Infinity, cMax = -Infinity;
  let sMin = Infinity, sMax = -Infinity;
  let hMin = Infinity, hMax = -Infinity;
  for (const c of candidates) {
    if (c.refinedOverhang < oMin) oMin = c.refinedOverhang;
    if (c.refinedOverhang > oMax) oMax = c.refinedOverhang;
    if (c.footprint < fMin) fMin = c.footprint;
    if (c.footprint > fMax) fMax = c.footprint;
    if (c.maxCross < cMin) cMin = c.maxCross;
    if (c.maxCross > cMax) cMax = c.maxCross;
    if (c.surfaceQuality < sMin) sMin = c.surfaceQuality;
    if (c.surfaceQuality > sMax) sMax = c.surfaceQuality;
    if (c.estHeight < hMin) hMin = c.estHeight;
    if (c.estHeight > hMax) hMax = c.estHeight;
  }
  const oSpan = Math.max(oMax - oMin, 1e-9);
  const fSpan = Math.max(fMax - fMin, 1e-9);
  const cSpan = Math.max(cMax - cMin, 1e-9);
  const sSpan = Math.max(sMax - sMin, 1e-9);
  const hSpan = Math.max(hMax - hMin, 1e-9);
  const ranked = candidates.map(c => {
    const on = (c.refinedOverhang - oMin) / oSpan;
    const fn = (c.footprint - fMin) / fSpan;
    const cn = (c.maxCross - cMin) / cSpan;
    // surfaceQuality is a MAXIMISE metric → invert so high quality = low cost.
    const sn = (sMax - c.surfaceQuality) / sSpan;
    const hn = (c.estHeight - hMin) / hSpan;
    return {
      ...c,
      compositeScore:
        weights.wOverhang * on +
        weights.wFootprint * fn +
        weights.wCross * cn +
        weights.wSurface * sn +
        weights.wHeight * hn,
    };
  });
  ranked.sort((a, b) => a.compositeScore - b.compositeScore);
  return ranked;
}

/** Consensus (minimax) ranking — each candidate's compositeScore is
 *  1 - max(normalised costs), so 1.0 = perfect (100%) and 0.0 = worst.
 *  Ordering favours candidates whose WORST metric is the best. Pure.
 *
 *  All five heuristics participate with equal voice. Maximise metrics
 *  (surfaceQuality) are inverted to cost form before the max. Height enters
 *  as its own cost term (shorter print = better). */
export function rankByConsensus(candidates: Candidate[]): Candidate[] {
  if (candidates.length === 0) return [];
  const norm = (vals: number[]) => {
    let lo = Infinity, hi = -Infinity;
    for (const v of vals) { if (v < lo) lo = v; if (v > hi) hi = v; }
    const span = Math.max(hi - lo, 1e-9);
    return vals.map(v => (v - lo) / span);
  };
  const invert = (vals: number[]) => {
    let lo = Infinity, hi = -Infinity;
    for (const v of vals) { if (v < lo) lo = v; if (v > hi) hi = v; }
    const span = Math.max(hi - lo, 1e-9);
    return vals.map(v => (hi - v) / span);
  };
  const oN = norm(candidates.map(c => c.refinedOverhang));
  const fN = norm(candidates.map(c => c.footprint));
  const cN = norm(candidates.map(c => c.maxCross));
  const sN = norm(candidates.map(c => c.shadowed));
  const qN = invert(candidates.map(c => c.surfaceQuality)); // maximise → invert
  const hN = norm(candidates.map(c => c.estHeight));
  const ranked = candidates.map((c, i) => ({
    ...c,
    compositeScore: 1 - Math.max(oN[i], fN[i], cN[i], sN[i], qN[i], hN[i]),
  }));
  ranked.sort((a, b) => b.compositeScore - a.compositeScore);
  return ranked;
}

/** TOPSIS MCDA ranker. Vector-normalises 5 metrics, multiplies by weights,
 *  computes Euclidean distance to ideal-best and ideal-worst, and ranks by
 *  closeness coefficient C_i = S-/(S+ + S-) ∈ [0,1] (higher = better).
 *
 *  Metrics: overhangPenalty (cost), footprint (cost), maxCross (cost),
 *  surfaceQuality (benefit), estHeight (cost). Columns with weight=0 are
 *  skipped entirely (contribute nothing to the distance). Pure — returns
 *  a NEW array, does not mutate input.
 *
 *  Per D-06: textbook TOPSIS with vector normalisation. */
export function rankByTopsis(candidates: Candidate[], weights: ScoreWeights): Candidate[] {
  if (candidates.length === 0) return [];
  const n = candidates.length;

  // Extract raw metric arrays.
  const over = candidates.map(c => c.refinedOverhang);
  const foot = candidates.map(c => c.footprint);
  const cross = candidates.map(c => c.maxCross);
  const surf = candidates.map(c => c.surfaceQuality);
  const height = candidates.map(c => c.estHeight);

  // Cost metric: lower = better. Benefit metric (surface): higher = better.
  // For vector normalisation: v_j = x_ij / sqrt(sum(x_kj^2)).
  function normCol(vals: number[]): number[] {
    let sq = 0;
    for (const v of vals) sq += v * v;
    const denom = Math.sqrt(sq);
    const d = Math.max(denom, 1e-9);
    return vals.map(v => v / d);
  }

  // Normalise each column.
  const oN = normCol(over);
  const fN = normCol(foot);
  const cN = normCol(cross);
  const sN = normCol(surf);
  const hN = normCol(height);

  // Apply weights. Skip columns where weight is 0 (all zeros contribution).
  const wO = weights.wOverhang, wF = weights.wFootprint, wC = weights.wCross;
  const wS = weights.wSurface, wH = weights.wHeight;

  const wo = oN.map(v => v * wO);
  const wf = fN.map(v => v * wF);
  const wc = cN.map(v => v * wC);
  const ws = sN.map(v => v * wS);
  const wh = hN.map(v => v * wH);

  // Ideal-best A+: min for cost metrics, max for benefit (surface).
  let oBest = Infinity, oWorst = -Infinity;
  let fBest = Infinity, fWorst = -Infinity;
  let cBest = Infinity, cWorst = -Infinity;
  let sBest = -Infinity, sWorst = Infinity;
  let hBest = Infinity, hWorst = -Infinity;
  for (let i = 0; i < n; i++) {
    if (wo[i] < oBest) oBest = wo[i]; if (wo[i] > oWorst) oWorst = wo[i];
    if (wf[i] < fBest) fBest = wf[i]; if (wf[i] > fWorst) fWorst = wf[i];
    if (wc[i] < cBest) cBest = wc[i]; if (wc[i] > cWorst) cWorst = wc[i];
    if (ws[i] > sBest) sBest = ws[i]; if (ws[i] < sWorst) sWorst = ws[i];
    if (wh[i] < hBest) hBest = wh[i]; if (wh[i] > hWorst) hWorst = wh[i];
  }

  // Compute S+ and S- for each candidate.
  const ranked = candidates.map((c, i) => {
    let sPlus = 0, sMinus = 0;
    if (wO > 0) { const d = wo[i] - oBest; sPlus += d * d; const dw = oWorst - wo[i]; sMinus += dw * dw; }
    if (wF > 0) { const d = wf[i] - fBest; sPlus += d * d; const dw = fWorst - wf[i]; sMinus += dw * dw; }
    if (wC > 0) { const d = wc[i] - cBest; sPlus += d * d; const dw = cWorst - wc[i]; sMinus += dw * dw; }
    if (wS > 0) { const d = sBest - ws[i]; sPlus += d * d; const dw = ws[i] - sWorst; sMinus += dw * dw; }
    if (wH > 0) { const d = wh[i] - hBest; sPlus += d * d; const dw = hWorst - wh[i]; sMinus += dw * dw; }

    sPlus = Math.sqrt(sPlus);
    sMinus = Math.sqrt(sMinus);

    // Closeness C_i = S- / (S+ + S-). If both are 0 (single candidate or
    // all-identical metrics on active columns), define C_i = 1.0.
    let closeness: number;
    if (sPlus + sMinus < 1e-12) {
      closeness = 1.0;
    } else {
      closeness = sMinus / (sPlus + sMinus);
    }

    return { ...c, compositeScore: closeness };
  });

  ranked.sort((a, b) => b.compositeScore - a.compositeScore);
  return ranked;
}

/** Find the nearest pre-computed candidate's composite score for a given
 *  quaternion orientation. Used during overlay drag for live score badge. */
export function nearestCandidateScore(
  quaternion: [number, number, number, number],
  candidates: Candidate[],
): { score: number; index: number } {
  const invQ: [number, number, number, number] = [quaternion[0], -quaternion[1], -quaternion[2], -quaternion[3]];
  const dir = applyQuaternion(invQ, [0, -1, 0]);
  let bestScore = 0;
  let bestIdx = 0;
  let bestDot = -Infinity;
  for (let i = 0; i < candidates.length; i++) {
    const c = candidates[i];
    const cInvQ: [number, number, number, number] = [c.quaternion[0], -c.quaternion[1], -c.quaternion[2], -c.quaternion[3]];
    const cDir = applyQuaternion(cInvQ, [0, -1, 0]);
    const dot = dir[0] * cDir[0] + dir[1] * cDir[1] + dir[2] * cDir[2];
    if (dot > bestDot) {
      bestDot = dot;
      bestScore = c.compositeScore;
      bestIdx = i;
    }
  }
  return { score: bestScore, index: bestIdx };
}

function applyQuaternion(q: [number, number, number, number], v: [number, number, number]): [number, number, number] {
  const [w, x, y, z] = q;
  const [vx, vy, vz] = v;
  const uv_x = y * vz - z * vy;
  const uv_y = z * vx - x * vz;
  const uv_z = x * vy - y * vx;
  const uuv_x = y * uv_z - z * uv_y;
  const uuv_y = z * uv_x - x * uv_z;
  const uuv_z = x * uv_y - y * uv_x;
  return [
    vx + 2.0 * (w * uv_x + uuv_x),
    vy + 2.0 * (w * uv_y + uuv_y),
    vz + 2.0 * (w * uv_z + uuv_z),
  ];
}
