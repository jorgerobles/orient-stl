import type { OriData } from './types';

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
