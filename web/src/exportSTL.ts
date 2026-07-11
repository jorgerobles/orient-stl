export function exportSTL(
  positions: Float32Array,
  name: string,
  candidateIndex: number,
): void {
  const triCount = positions.length / 9;
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
    const a: [number, number, number] = [positions[base], positions[base + 1], positions[base + 2]];
    const b: [number, number, number] = [positions[base + 3], positions[base + 4], positions[base + 5]];
    const c: [number, number, number] = [positions[base + 6], positions[base + 7], positions[base + 8]];
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
  link.download = `${baseName}_orient_${candidateIndex}.stl`;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
