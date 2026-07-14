import * as THREE from "three";
import { RING_SCALE, CAMERA_RING_SCALE, TUBE_MIN_RATIO } from "../constants";

export type RingAxis = "axis-x" | "axis-y" | "axis-z" | "camera";

export class GizmoController {
  readonly group: THREE.Group;
  private ringX: THREE.Mesh | null = null;
  private ringY: THREE.Mesh | null = null;
  private ringZ: THREE.Mesh | null = null;
  private cameraRing: THREE.Mesh | null = null;
  private raycaster = new THREE.Raycaster();
  private hoveredRing: RingAxis | null = null;
  private ringDefaultOpacities = new Map<THREE.Mesh, number>();
  private boundingRadius: number;

  constructor(modelPosition: THREE.Vector3, boundingRadius: number) {
    this.group = new THREE.Group();
    this.group.position.copy(modelPosition);
    this.boundingRadius = boundingRadius;
    this.createGizmo();
  }

  private makeRing(
    radius: number,
    tube: number,
    color: number,
    opacity = 0.7,
  ): THREE.Mesh {
    const geometry = new THREE.TorusGeometry(radius, tube, 12, 48);
    const material = new THREE.MeshBasicMaterial({
      color,
      transparent: true,
      opacity,
      depthWrite: false,
    });
    const mesh = new THREE.Mesh(geometry, material);
    this.ringDefaultOpacities.set(mesh, opacity);
    return mesh;
  }

  private createGizmo(): void {
    const r = this.boundingRadius * RING_SCALE;
    const tube = Math.max(r * TUBE_MIN_RATIO, 0.02);

    this.ringX = this.makeRing(r, tube, 0xff4444);
    this.ringX.rotation.y = Math.PI / 2;
    this.group.add(this.ringX);

    this.ringY = this.makeRing(r, tube, 0x44ff44);
    this.group.add(this.ringY);

    this.ringZ = this.makeRing(r, tube, 0x4488ff);
    this.ringZ.rotation.x = Math.PI / 2;
    this.group.add(this.ringZ);

    // Camera ring — outer, white, always faces camera via billboard
    const cr = this.boundingRadius * CAMERA_RING_SCALE;
    const ct = Math.max(cr * 0.004, 0.015);
    this.cameraRing = this.makeRing(cr, ct, 0xcccccc, 0.35);
    this.group.add(this.cameraRing);
  }

  billboard(camera: THREE.Camera): void {
    if (!this.cameraRing) return;
    const worldPos = new THREE.Vector3();
    this.cameraRing.getWorldPosition(worldPos);
    const dir = new THREE.Vector3()
      .subVectors(camera.position, worldPos)
      .normalize();
    this.cameraRing.quaternion.setFromUnitVectors(
      new THREE.Vector3(0, 0, 1),
      dir,
    );
  }

  raycastRing(
    ndc: THREE.Vector2,
    camera: THREE.Camera,
  ): RingAxis | null {
    this.raycaster.setFromCamera(ndc, camera);

    const meshes: { mesh: THREE.Mesh; mode: RingAxis }[] = [];
    if (this.ringX) meshes.push({ mesh: this.ringX, mode: "axis-x" });
    if (this.ringY) meshes.push({ mesh: this.ringY, mode: "axis-y" });
    if (this.ringZ) meshes.push({ mesh: this.ringZ, mode: "axis-z" });
    if (this.cameraRing)
      meshes.push({ mesh: this.cameraRing, mode: "camera" });

    const hits = this.raycaster.intersectObjects(meshes.map((m) => m.mesh));
    if (hits.length > 0) {
      for (const m of meshes) {
        if (hits[0].object === m.mesh) return m.mode;
      }
    }
    return null;
  }

  setHover(mode: RingAxis | null): void {
    // Restore previous ring's opacity
    if (this.hoveredRing !== null) {
      const prev = this.getRingMesh(this.hoveredRing);
      if (prev) {
        const orig = this.ringDefaultOpacities.get(prev) ?? 0.7;
        (prev.material as THREE.MeshBasicMaterial).opacity = orig;
      }
    }
    // Highlight new ring
    if (mode) {
      const ring = this.getRingMesh(mode);
      if (ring)
        (ring.material as THREE.MeshBasicMaterial).opacity = 1.0;
    }
    this.hoveredRing = mode;
  }

  private getRingMesh(mode: RingAxis): THREE.Mesh | null {
    switch (mode) {
      case "axis-x":
        return this.ringX;
      case "axis-y":
        return this.ringY;
      case "axis-z":
        return this.ringZ;
      case "camera":
        return this.cameraRing;
    }
  }

  dispose(): void {
    for (let i = this.group.children.length - 1; i >= 0; i--) {
      const child = this.group.children[i];
      this.group.remove(child);
      if (child instanceof THREE.Mesh) {
        child.geometry.dispose();
        (child.material as THREE.Material).dispose();
      }
    }
    this.ringX = null;
    this.ringY = null;
    this.ringZ = null;
    this.cameraRing = null;
    this.ringDefaultOpacities.clear();
    this.hoveredRing = null;
  }
}
