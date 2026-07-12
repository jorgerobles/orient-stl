export type LoadConvention = "z-up" | "y-up";

/**
 * Apply the load-time axis convention to a flat xyz array.
 *
 * The tool's internal frame is Y-up: the build plate lies in the XZ plane and
 * +Y is the plate normal (see centering.ts → liftOntoPlate). STL files arrive
 * in one of two conventions, selected by the user at load time:
 *
 *   'z-up' — STL's +Z is the vertical axis (most CAD output, default for most
 *            slicers). Rotate -90° about X so STL+Z → tool+Y:
 *            (x, y, z) → (x, z, -y). This is a proper rotation (preserves
 *            handedness and lengths) — not a mirror.
 *
 *   'y-up' — STL's +Y is already the vertical axis (e.g. Blender, three.js
 *            exports). No rotation needed; the input is returned as-is.
 *
 * The same function applies to positions, face normals, AND candidate
 * direction vectors — all are flat xyz arrays in the same frame, so calling
 * applyConvention on each keeps the scoring pipeline in a consistent frame.
 *
 * Returns a new array for 'z-up'; returns the SAME array (no copy) for 'y-up'
 * since it is a true no-op. Never mutates the input.
 */
export function applyConvention(
  coords: Float32Array,
  convention: LoadConvention,
): Float32Array {
  if (convention === "y-up") return coords;

  // 'z-up': swap Y and Z with a sign flip to preserve handedness.
  // Equivalent to quaternion (-√½, 0, 0, √½) — a -90° rotation about X.
  const out = new Float32Array(coords.length);
  for (let i = 0; i < coords.length; i += 3) {
    out[i] = coords[i];
    out[i + 1] = coords[i + 2];
    out[i + 2] = -coords[i + 1];
  }
  return out;
}
