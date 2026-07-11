// Reference: binary STL format validation
// Run: node stl-format-ref.js
// This is a JS reference to validate our understanding of binary STL,
// so the vendor parser path (if needed) is grounded in tested format knowledge.

const assert = require('assert');

function createBinarySTL(triangles) {
  // 80-byte header
  const header = Buffer.alloc(80);
  header.write('binary stl reference', 0, 20, 'ascii');

  // 4-byte triangle count
  const count = Buffer.alloc(4);
  count.writeUInt32LE(triangles.length, 0);

  const buffers = [header, count];

  for (const tri of triangles) {
    const triBuf = Buffer.alloc(50);
    let offset = 0;

    // normal (3 x f32 LE)
    triBuf.writeFloatLE(tri.normal[0], offset); offset += 4;
    triBuf.writeFloatLE(tri.normal[1], offset); offset += 4;
    triBuf.writeFloatLE(tri.normal[2], offset); offset += 4;

    // vertex 1
    triBuf.writeFloatLE(tri.v1[0], offset); offset += 4;
    triBuf.writeFloatLE(tri.v1[1], offset); offset += 4;
    triBuf.writeFloatLE(tri.v1[2], offset); offset += 4;

    // vertex 2
    triBuf.writeFloatLE(tri.v2[0], offset); offset += 4;
    triBuf.writeFloatLE(tri.v2[1], offset); offset += 4;
    triBuf.writeFloatLE(tri.v2[2], offset); offset += 4;

    // vertex 3
    triBuf.writeFloatLE(tri.v3[0], offset); offset += 4;
    triBuf.writeFloatLE(tri.v3[1], offset); offset += 4;
    triBuf.writeFloatLE(tri.v3[2], offset); offset += 4;

    // attribute byte count (2 bytes, usually 0)
    triBuf.writeUInt16LE(0, offset);

    buffers.push(triBuf);
  }

  return Buffer.concat(buffers);
}

function parseBinarySTL(buffer) {
  assert(buffer.length >= 84, 'File too small for header + count');

  const count = buffer.readUInt32LE(80);
  const expectedSize = 84 + count * 50;
  assert(buffer.length >= expectedSize, `Expected ${expectedSize} bytes, got ${buffer.length}`);

  const triangles = [];
  let offset = 84;

  for (let i = 0; i < count; i++) {
    const tri = { normal: [], v1: [], v2: [], v3: [] };

    tri.normal = [
      buffer.readFloatLE(offset), buffer.readFloatLE(offset + 4), buffer.readFloatLE(offset + 8),
    ];
    offset += 12;

    tri.v1 = [
      buffer.readFloatLE(offset), buffer.readFloatLE(offset + 4), buffer.readFloatLE(offset + 8),
    ];
    offset += 12;

    tri.v2 = [
      buffer.readFloatLE(offset), buffer.readFloatLE(offset + 4), buffer.readFloatLE(offset + 8),
    ];
    offset += 12;

    tri.v3 = [
      buffer.readFloatLE(offset), buffer.readFloatLE(offset + 4), buffer.readFloatLE(offset + 8),
    ];
    offset += 12;

    // skip attribute byte count (2 bytes)
    offset += 2;

    triangles.push(tri);
  }

  return triangles;
}

// Test: create a simple 2-triangle STL (a quad)
const triangles = [
  {
    normal: [0, 0, 1],
    v1: [-1, -1, 0],
    v2: [1, -1, 0],
    v3: [1, 1, 0],
  },
  {
    normal: [0, 0, 1],
    v1: [-1, -1, 0],
    v2: [1, 1, 0],
    v3: [-1, 1, 0],
  },
];

const buf = createBinarySTL(triangles);

// Verify roundtrip
const parsed = parseBinarySTL(buf);
assert.strictEqual(parsed.length, 2, 'Should parse 2 triangles');

// Check values
assert.deepStrictEqual(parsed[0].normal, [0, 0, 1]);
assert.deepStrictEqual(parsed[0].v1, [-1, -1, 0]);
assert.deepStrictEqual(parsed[0].v2, [1, -1, 0]);
assert.deepStrictEqual(parsed[0].v3, [1, 1, 0]);

// Verify expected file size: 84 + 2 * 50 = 184 bytes
assert.strictEqual(buf.length, 184, `Expected 184 bytes, got ${buf.length}`);

console.log('✓ Binary STL roundtrip OK (184 bytes for 2 triangles)');
console.log('  Format: 80-byte header | u32 count | N × 50-byte triangles');
console.log('  Each tri: normal(12) + v1(12) + v2(12) + v3(12) + attrib(2) = 50 bytes');
