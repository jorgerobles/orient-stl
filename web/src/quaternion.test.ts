import { describe, it, expect } from "vitest";
import { applyQuat, invQuatFromXYZW, dirFromQuat } from "./quaternion";

// Build a quaternion [x, y, z, w] for a rotation about an axis.
function quatFromAxisAngle(ax: number, ay: number, az: number, deg: number): [number, number, number, number] {
  const nlen = Math.sqrt(ax*ax + ay*ay + az*az);
  const nx = ax / nlen, ny = ay / nlen, nz = az / nlen;
  const half = deg * Math.PI / 360;
  const s = Math.sin(half);
  const c = Math.cos(half);
  return [nx * s, ny * s, nz * s, c]; // [x, y, z, w]
}

const len = (v: number[]) => Math.sqrt(v[0]*v[0] + v[1]*v[1] + v[2]*v[2]);

describe("dirFromQuat — identity", () => {
  it("identity quaternion gives [0, -1, 0] (down)", () => {
    const dir = dirFromQuat([0, 0, 0, 1]);
    expect(dir[0]).toBeCloseTo(0, 5);
    expect(dir[1]).toBeCloseTo(-1, 5);
    expect(dir[2]).toBeCloseTo(0, 5);
  });
});

describe("dirFromQuat — 90° rotations (right-hand rule)", () => {
  it("90° about X: model +Z → world -Y, so world -Y maps to model +Z", () => {
    const dir = dirFromQuat(quatFromAxisAngle(1, 0, 0, 90));
    expect(dir[0]).toBeCloseTo(0, 5);
    expect(dir[1]).toBeCloseTo(0, 5);
    expect(dir[2]).toBeCloseTo(1, 5);
  });

  it("90° about Y: rotating around vertical axis, down stays down", () => {
    const dir = dirFromQuat(quatFromAxisAngle(0, 1, 0, 90));
    expect(dir[0]).toBeCloseTo(0, 5);
    expect(dir[1]).toBeCloseTo(-1, 5);
    expect(dir[2]).toBeCloseTo(0, 5);
  });

  it("90° about Z: model -X → world -Y, so world -Y maps to model -X", () => {
    const dir = dirFromQuat(quatFromAxisAngle(0, 0, 1, 90));
    expect(dir[0]).toBeCloseTo(-1, 5);
    expect(dir[1]).toBeCloseTo(0, 5);
    expect(dir[2]).toBeCloseTo(0, 5);
  });
});

describe("dirFromQuat — 180° rotations", () => {
  it("180° about X flips -Y → +Y (upside down)", () => {
    const dir = dirFromQuat(quatFromAxisAngle(1, 0, 0, 180));
    expect(dir[0]).toBeCloseTo(0, 5);
    expect(dir[1]).toBeCloseTo(1, 5);
    expect(dir[2]).toBeCloseTo(0, 5);
  });

  it("180° about Z flips -Y → +Y", () => {
    const dir = dirFromQuat(quatFromAxisAngle(0, 0, 1, 180));
    expect(dir[0]).toBeCloseTo(0, 5);
    expect(dir[1]).toBeCloseTo(1, 5);
    expect(dir[2]).toBeCloseTo(0, 5);
  });
});

describe("dirFromQuat — output is unit length", () => {
  it("always returns a unit vector", () => {
    for (const deg of [0, 15, 30, 45, 90, 137, 180]) {
      const q = quatFromAxisAngle(1, 0.7, 0.3, deg);
      const dir = dirFromQuat(q);
      expect(len(dir)).toBeCloseTo(1, 4);
    }
  });
});

describe("invQuatFromXYZW — inverse correctness", () => {
  it("applying a rotation then its inverse returns the original vector", () => {
    const deg = 37;
    const half = deg * Math.PI / 360;
    const s = Math.sin(half), c = Math.cos(half);
    const qXYZW: [number, number, number, number] = [s, 0, 0, c];
    const qWXYZ: [number, number, number, number] = [c, s, 0, 0];
    const original: [number, number, number] = [0, -1, 0];
    const rotated = applyQuat(qWXYZ, original);
    const inv = invQuatFromXYZW(qXYZW);
    const back = applyQuat(inv, rotated);
    expect(back[0]).toBeCloseTo(original[0], 4);
    expect(back[1]).toBeCloseTo(original[1], 4);
    expect(back[2]).toBeCloseTo(original[2], 4);
  });
});
