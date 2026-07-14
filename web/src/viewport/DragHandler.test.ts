import { describe, it, expect, vi } from "vitest";
import { DragHandler } from "./DragHandler";

// Minimal THREE.Vector3 mock that lets us test getAxisVector
function mockVec3(x = 0, y = 0, z = 0) {
  return { x, y, z, normalize: vi.fn() };
}

function createMocks() {
  const camera = {
    getWorldDirection: vi.fn(() => mockVec3(0, 0, -1)),
  };
  const mesh = { quaternion: { clone: vi.fn(() => ({ x: 0, y: 0, z: 0, w: 1 })) } };
  const controls = { enabled: true };
  const dummyVec3 = { x: 0, y: 0, z: 0 };
  const gizmo = {
    raycastRing: vi.fn(),
    setHover: vi.fn(),
    group: { position: dummyVec3 },
  };
  const domElement = {
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    setPointerCapture: vi.fn(),
    getBoundingClientRect: vi.fn(() => ({ left: 0, top: 0, width: 800, height: 600 }) as DOMRect),
  } as unknown as HTMLCanvasElement;
  const onOrientationChange = vi.fn();
  return { camera, mesh, controls, gizmo, domElement, onOrientationChange };
}

describe("DragHandler", () => {
  it("getAxisVector('axis-x') returns [1, 0, 0]", () => {
    const mocks = createMocks();
    const handler = new DragHandler(
      mocks.gizmo as never,
      mocks.mesh as never,
      mocks.camera as never,
      mocks.domElement,
      10,
      mocks.controls as never,
      mocks.onOrientationChange,
    );
    expect(handler.getAxisVector("axis-x")).toEqual([1, 0, 0]);
  });

  it("getAxisVector('axis-y') returns [0, 0, 1] — current behavior (Pitfall 3 pin)", () => {
    const mocks = createMocks();
    const handler = new DragHandler(
      mocks.gizmo as never,
      mocks.mesh as never,
      mocks.camera as never,
      mocks.domElement,
      10,
      mocks.controls as never,
      mocks.onOrientationChange,
    );
    expect(handler.getAxisVector("axis-y")).toEqual([0, 0, 1]);
  });

  it("getAxisVector('axis-z') returns [0, 1, 0] — current behavior (Pitfall 3 pin)", () => {
    const mocks = createMocks();
    const handler = new DragHandler(
      mocks.gizmo as never,
      mocks.mesh as never,
      mocks.camera as never,
      mocks.domElement,
      10,
      mocks.controls as never,
      mocks.onOrientationChange,
    );
    expect(handler.getAxisVector("axis-z")).toEqual([0, 1, 0]);
  });

  it("dispose() removes all pointer event listeners", () => {
    const mocks = createMocks();
    const handler = new DragHandler(
      mocks.gizmo as never,
      mocks.mesh as never,
      mocks.camera as never,
      mocks.domElement,
      10,
      mocks.controls as never,
      mocks.onOrientationChange,
    );

    const addListener = vi.mocked(mocks.domElement.addEventListener);
    const removeListener = vi.mocked(mocks.domElement.removeEventListener);

    // Constructor should have added 3 listeners
    expect(addListener).toHaveBeenCalledTimes(3);

    handler.dispose();

    // dispose should remove all 3 listeners
    expect(removeListener).toHaveBeenCalledTimes(3);
    // Verify event types match
    const addedEvents = addListener.mock.calls.map(
      (c: unknown[]) => c[0],
    );
    const removedEvents = removeListener.mock.calls.map(
      (c: unknown[]) => c[0],
    );
    expect(removedEvents).toEqual(addedEvents);
  });
});
