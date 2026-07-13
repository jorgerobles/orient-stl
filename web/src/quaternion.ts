/** Quaternion utilities for orientation scoring.
 *
 *  THREE.Quaternion stores components as (x, y, z, w). The viewport returns
 *  [x, y, z, w] via getMeshQuaternion(). This module works in that same order.
 *
 *  The "down" direction in model space for a given mesh orientation is found
 *  by applying the INVERSE of the mesh quaternion to world-space [0, -1, 0]. */

/** Apply quaternion q = [w, x, y, z] to vector v = [vx, vy, vz]. */
export function applyQuat(
  q: [number, number, number, number],
  v: [number, number, number],
): [number, number, number] {
  const [w, x, y, z] = q;
  const [vx, vy, vz] = v;
  const uv_x = y * vz - z * vy;
  const uv_y = z * vx - x * vz;
  const uv_z = x * vy - y * vx;
  const uuv_x = y * uv_z - z * uv_y;
  const uuv_y = z * uv_x - x * uv_z;
  const uuv_z = x * uv_y - y * uv_x;
  return [
    vx + 2 * (w * uv_x + uuv_x),
    vy + 2 * (w * uv_y + uuv_y),
    vz + 2 * (w * uv_z + uuv_z),
  ];
}

/** Inverse of a unit quaternion given as [x, y, z, w].
 *  Returns [w, -x, -y, -z] — ready for applyQuat. */
export function invQuatFromXYZW(
  q: [number, number, number, number],
): [number, number, number, number] {
  return [q[3], -q[0], -q[1], -q[2]];
}

/** Full pipeline: mesh quaternion [x,y,z,w] → "down" direction in model space.
 *  This is the direction that world-space -Y maps to in the model's local frame,
 *  i.e. the direction that points toward the build plate. */
export function dirFromQuat(
  q: [number, number, number, number],
): [number, number, number] {
  return applyQuat(invQuatFromXYZW(q), [0, -1, 0]);
}
