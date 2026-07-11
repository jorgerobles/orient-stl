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
 * Y-axis lift needed to rest the lowest point of a centroid-centered,
 * already-rotated mesh on the build plate at y=0. Apply to
 * `modelGroup.position.y`. Never sinks a model that already floats.
 *
 * X and Z are left at 0: because the mesh rotates around its centroid (now the
 * local origin) and modelGroup sits at the world origin, the rotated centroid
 * stays at (0, _, 0) — horizontally centered for every candidate.
 */
export function liftOntoPlate(rotatedMinY: number): number {
  return rotatedMinY < 0 ? -rotatedMinY : 0;
}
