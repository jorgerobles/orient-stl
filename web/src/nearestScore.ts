import type { Candidate } from './compute';

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
