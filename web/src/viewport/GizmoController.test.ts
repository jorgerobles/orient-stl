import { describe, it, expect, vi } from "vitest";
import { GizmoController } from "./GizmoController";

function createGizmo() {
  const pos = { x: 0, y: 10, z: 0 };
  return new GizmoController(pos as never, 10);
}

describe("GizmoController", () => {
  it("raycastRing returns null when no rings are hit (empty raycaster mock)", () => {
    const gizmo = createGizmo();

    // Replace internal raycaster with a mock that returns no hits
    const mockRaycaster = {
      setFromCamera: vi.fn(),
      intersectObjects: vi.fn(() => []),
    };
    (gizmo as unknown as { raycaster: typeof mockRaycaster }).raycaster =
      mockRaycaster;

    const ndc = { x: 0, y: 0 };
    const camera = { position: { x: 0, y: 0, z: 10 } };
    const result = gizmo.raycastRing(ndc as never, camera as never);
    expect(result).toBeNull();
    expect(mockRaycaster.setFromCamera).toHaveBeenCalledWith(ndc, camera);
    expect(mockRaycaster.intersectObjects).toHaveBeenCalled();
  });

  it("setHover sets ring material opacity to 1.0 for highlighted ring, restores default on null", () => {
    const gizmo = createGizmo();

    // Initially no hover
    // Simulate setHover for 'axis-x'
    // We need to check that the ring material opacity changes.
    // Access the internal ringX via the group traversal or by checking the type.
    // Since group.children[0] is ringX (created first), we can find it.
    const ringX = gizmo.group.children.find(
      (c) => "material" in c && "isMesh" in c,
    ) as unknown as {
      material: { opacity: number };
    };

    if (ringX) {
      const origOpacity = ringX.material.opacity;

      // Set hover to axis-x
      gizmo.setHover("axis-x" as never);
      expect(ringX.material.opacity).toBe(1.0);

      // Clear hover — should restore original opacity
      gizmo.setHover(null as never);
      expect(ringX.material.opacity).toBe(origOpacity);
    }
  });

  it("dispose removes all children from its group", () => {
    const gizmo = createGizmo();
    expect(gizmo.group.children.length).toBeGreaterThan(0);

    gizmo.dispose();
    expect(gizmo.group.children.length).toBe(0);
  });
});
