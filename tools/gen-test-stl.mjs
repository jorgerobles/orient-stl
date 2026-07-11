import fs from 'fs';

const header = new Uint8Array(80);
const triCount = 4;
const buf = new DataView(new ArrayBuffer(84 + triCount * 50));
header.forEach((b, i) => buf.setUint8(i, b));
buf.setUint32(80, triCount, true);

const v = [
  [-10, -10, -10],
  [10, -10, -10],
  [0, 10, -10],
  [0, 0, 15],
];

function writeTri(dv, off, a, b, c) {
  const e1 = [b[0]-a[0], b[1]-a[1], b[2]-a[2]];
  const e2 = [c[0]-a[0], c[1]-a[1], c[2]-a[2]];
  const nx = e1[1]*e2[2] - e1[2]*e2[1];
  const ny = e1[2]*e2[0] - e1[0]*e2[2];
  const nz = e1[0]*e2[1] - e1[1]*e2[0];
  const len = Math.sqrt(nx*nx + ny*ny + nz*nz);
  dv.setFloat32(off, nx/len, true); dv.setFloat32(off+4, ny/len, true); dv.setFloat32(off+8, nz/len, true);
  const base = off + 12;
  [a, b, c].forEach((p, i) => {
    dv.setFloat32(base + i*12, p[0], true);
    dv.setFloat32(base + i*12 + 4, p[1], true);
    dv.setFloat32(base + i*12 + 8, p[2], true);
  });
  dv.setUint16(off + 48, 0, true);
}

const faces = [[0,1,2],[0,2,3],[0,3,1],[1,3,2]];
faces.forEach(([a,b,c], i) => writeTri(buf, 84 + i*50, v[a], v[b], v[c]));

fs.writeFileSync('test-tetrahedron.stl', Buffer.from(buf.buffer));
console.log('Wrote test-tetrahedron.stl');
