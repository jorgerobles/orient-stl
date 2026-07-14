export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

/**
 * Translation to bake into the geometry so the mesh's centroid sits at the
 * local origin. With the centroid at the local origin, `mesh.quaternion`
 * rotates the mesh around its centroid (in-place spin) instead of around an
 * arbitrary corner of the model (orbit).
 *
 * Bake this ONCE at load time via `geometry.translate(...)`; do NOT apply it
 * as a group offset, because a group offset does not rotate with the mesh.
 */
export function centroidTranslate(centroid: Vec3): Vec3 {
  return { x: -centroid.x, y: -centroid.y, z: -centroid.z };
}

/**
 * Max distance from `centroid` to any vertex in `positions`. The result is
 * the bounding-sphere radius of the mesh around its centroid.
 * O(n) — iterates once, compares squared distances, one sqrt at the end.
 * Works correctly before or after baking the centroid into the geometry.
 */
export function boundingRadius(centroid: Vec3, positions: Float32Array): number {
  let maxR2 = 0;
  for (let i = 0; i < positions.length; i += 3) {
    const dx = positions[i] - centroid.x;
    const dy = positions[i + 1] - centroid.y;
    const dz = positions[i + 2] - centroid.z;
    const r2 = dx * dx + dy * dy + dz * dz;
    if (r2 > maxR2) maxR2 = r2;
  }
  return Math.sqrt(maxR2);
}
