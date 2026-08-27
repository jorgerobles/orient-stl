import * as THREE from 'three';
import type { SupportRenderer } from './viewport/SupportRenderer';

export function exportSTL(
  positions: Float32Array,
  name: string,
  candidateIndex: number,
  supportRenderer?: SupportRenderer | null,
): void {
  const allPositions: number[] = [];

  for (let i = 0; i < positions.length; i++) {
    allPositions.push(positions[i]);
  }

  if (supportRenderer?.isVisible) {
    for (const column of supportRenderer.getColumnMeshes()) {
      const geo = column.geometry;
      const posAttr = geo.attributes.position as THREE.BufferAttribute;
      const idx = geo.index;
      column.updateMatrixWorld();
      const matrix = column.matrixWorld;

      if (idx) {
        for (let i = 0; i < idx.count; i += 3) {
          for (let j = 0; j < 3; j++) {
            const vi = idx.getX(i + j);
            const v = new THREE.Vector3(
              posAttr.getX(vi),
              posAttr.getY(vi),
              posAttr.getZ(vi),
            );
            v.applyMatrix4(matrix);
            allPositions.push(v.x, v.y, v.z);
          }
        }
      } else {
        for (let i = 0; i < posAttr.count; i += 3) {
          for (let j = 0; j < 3; j++) {
            const v = new THREE.Vector3(
              posAttr.getX(i + j),
              posAttr.getY(i + j),
              posAttr.getZ(i + j),
            );
            v.applyMatrix4(matrix);
            allPositions.push(v.x, v.y, v.z);
          }
        }
      }
    }

    const raft = supportRenderer.getRaftMesh();
    if (raft) {
      const geo = raft.geometry;
      const posAttr = geo.attributes.position as THREE.BufferAttribute;
      const idx = geo.index;
      raft.updateMatrixWorld();
      const matrix = raft.matrixWorld;

      if (idx) {
        for (let i = 0; i < idx.count; i += 3) {
          for (let j = 0; j < 3; j++) {
            const vi = idx.getX(i + j);
            const v = new THREE.Vector3(
              posAttr.getX(vi),
              posAttr.getY(vi),
              posAttr.getZ(vi),
            );
            v.applyMatrix4(matrix);
            allPositions.push(v.x, v.y, v.z);
          }
        }
      } else {
        for (let i = 0; i < posAttr.count; i += 3) {
          for (let j = 0; j < 3; j++) {
            const v = new THREE.Vector3(
              posAttr.getX(i + j),
              posAttr.getY(i + j),
              posAttr.getZ(i + j),
            );
            v.applyMatrix4(matrix);
            allPositions.push(v.x, v.y, v.z);
          }
        }
      }
    }
  }

  const mergedPositions = new Float32Array(allPositions);
  const triCount = mergedPositions.length / 9;
  const header = new Uint8Array(80);
  const headerStr = `Orient STL candidate #${candidateIndex}`;
  for (let i = 0; i < headerStr.length && i < 80; i++) {
    header[i] = headerStr.charCodeAt(i);
  }
  const buf = new ArrayBuffer(84 + triCount * 50);
  const view = new DataView(buf);
  for (let i = 0; i < 80; i++) view.setUint8(i, header[i]);
  view.setUint32(80, triCount, true);

  for (let t = 0; t < triCount; t++) {
    const base = t * 9;
    const a: [number, number, number] = [mergedPositions[base], mergedPositions[base + 1], mergedPositions[base + 2]];
    const b: [number, number, number] = [mergedPositions[base + 3], mergedPositions[base + 4], mergedPositions[base + 5]];
    const c: [number, number, number] = [mergedPositions[base + 6], mergedPositions[base + 7], mergedPositions[base + 8]];
    const ex = b[0] - a[0], ey = b[1] - a[1], ez = b[2] - a[2];
    const fx = c[0] - a[0], fy = c[1] - a[1], fz = c[2] - a[2];
    let nx = ey * fz - ez * fy;
    let ny = ez * fx - ex * fz;
    let nz = ex * fy - ey * fx;
    const len = Math.sqrt(nx * nx + ny * ny + nz * nz);
    if (len > 1e-8) { nx /= len; ny /= len; nz /= len; }
    const off = 84 + t * 50;
    view.setFloat32(off, nx, true);
    view.setFloat32(off + 4, ny, true);
    view.setFloat32(off + 8, nz, true);
    view.setFloat32(off + 12, a[0], true);
    view.setFloat32(off + 16, a[1], true);
    view.setFloat32(off + 20, a[2], true);
    view.setFloat32(off + 24, b[0], true);
    view.setFloat32(off + 28, b[1], true);
    view.setFloat32(off + 32, b[2], true);
    view.setFloat32(off + 36, c[0], true);
    view.setFloat32(off + 40, c[1], true);
    view.setFloat32(off + 44, c[2], true);
    view.setUint16(off + 48, 0, true);
  }

  const blob = new Blob([buf], { type: 'application/octet-stream' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  const baseName = name.replace(/\.stl$/i, '');
  const suffix = supportRenderer?.isVisible ? '_with_supports' : '';
  link.download = `${baseName}_orient_${candidateIndex}${suffix}.stl`;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
