import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import type { GizmoController, RingAxis } from "./GizmoController";

export class DragHandler {
  private dragMode: RingAxis | null = null;
  private overlayStartQuat: THREE.Quaternion | null = null;
  private dragAxisVec: THREE.Vector3 | null = null;
  private prevIntersect: THREE.Vector3 | null = null;
  private cumulativeAngle = 0;
  private raycaster = new THREE.Raycaster();

  private boundOnDown = (e: PointerEvent) => this.onDown(e);
  private boundOnMove = (e: PointerEvent) => this.onMove(e);
  private boundOnUp = (e: PointerEvent) => this.onUp(e);

  constructor(
    private gizmo: GizmoController,
    private mesh: THREE.Mesh,
    private camera: THREE.PerspectiveCamera,
    private domElement: HTMLCanvasElement,
    private boundingRadius: number,
    private controls: OrbitControls,
    private onOrientationChange: (
      q: [number, number, number, number],
    ) => void,
  ) {
    this.attachHandlers();
  }

  /**
   * Return the world-axis vector for a drag mode.
   *
   * NOTE: The axis-y/axis-z mapping is INTENTIONAL and reflects current
   * runtime behavior. See Phase 6 Pitfall 3 regression test.
   * Investigating the mapping is a separate task.        -- Pitfall 3
   */
  getAxisVector(mode: RingAxis): [number, number, number] {
    switch (mode) {
      case "axis-x":
        return [1, 0, 0];
      case "axis-y":
        return [0, 0, 1]; // Intentional: axis-y ring rotates around world Z
      case "axis-z":
        return [0, 1, 0]; // Intentional: axis-z ring rotates around world Y
      case "camera": {
        const dir = new THREE.Vector3();
        this.camera.getWorldDirection(dir);
        dir.normalize();
        return [dir.x, dir.y, dir.z];
      }
    }
  }

  private attachHandlers(): void {
    this.domElement.addEventListener("pointerdown", this.boundOnDown, {
      capture: true,
    });
    this.domElement.addEventListener("pointermove", this.boundOnMove, {
      capture: true,
    });
    this.domElement.addEventListener("pointerup", this.boundOnUp, {
      capture: true,
    });
  }

  private getNDC(
    clientX: number,
    clientY: number,
  ): THREE.Vector2 {
    const rect = this.domElement.getBoundingClientRect();
    return new THREE.Vector2(
      ((clientX - rect.left) / rect.width) * 2 - 1,
      -((clientY - rect.top) / rect.height) * 2 + 1,
    );
  }

  private intersectRingPlane(
    clientX: number,
    clientY: number,
    axisVec: THREE.Vector3,
  ): THREE.Vector3 | null {
    const mouse = this.getNDC(clientX, clientY);
    this.raycaster.setFromCamera(mouse, this.camera);
    const center = new THREE.Vector3(0, this.boundingRadius, 0);
    const plane = new THREE.Plane().setFromNormalAndCoplanarPoint(
      axisVec,
      center,
    );
    const pt = new THREE.Vector3();
    return this.raycaster.ray.intersectPlane(plane, pt) ? pt : null;
  }

  private angleAroundAxis(
    pt: THREE.Vector3,
    axis: THREE.Vector3,
  ): number {
    const center = new THREE.Vector3(0, this.boundingRadius, 0);
    const dir = new THREE.Vector3().subVectors(pt, center);
    const proj = dir
      .clone()
      .sub(axis.clone().multiplyScalar(dir.dot(axis)));
    if (proj.lengthSq() < 0.0001) return 0;
    proj.normalize();
    const refBase =
      Math.abs(axis.y) > 0.9
        ? new THREE.Vector3(1, 0, 0)
        : Math.abs(axis.x) > 0.9
          ? new THREE.Vector3(0, 0, 1)
          : new THREE.Vector3(1, 0, 0);
    const ref = refBase
      .clone()
      .sub(axis.clone().multiplyScalar(refBase.dot(axis)));
    ref.normalize();
    return Math.atan2(
      axis.dot(proj.clone().cross(ref)),
      proj.dot(ref),
    );
  }

  private onDown(e: PointerEvent): void {
    const ndc = this.getNDC(e.clientX, e.clientY);
    const mode = this.gizmo.raycastRing(ndc, this.camera);
    if (!mode) return;
    e.stopPropagation();
    this.dragMode = mode;
    this.overlayStartQuat = this.mesh.quaternion.clone();
    this.domElement.setPointerCapture(e.pointerId);
    this.controls.enabled = false;

    const axisVec = new THREE.Vector3();
    const mv = this.getAxisVector(mode);
    axisVec.set(mv[0], mv[1], mv[2]);
    this.dragAxisVec = axisVec;
    this.prevIntersect = this.intersectRingPlane(
      e.clientX,
      e.clientY,
      axisVec,
    );
    this.cumulativeAngle = 0;
  }

  private onMove(e: PointerEvent): void {
    // Hover highlighting
    if (!this.dragMode) {
      const ndc = this.getNDC(e.clientX, e.clientY);
      const mode = this.gizmo.raycastRing(ndc, this.camera);
      this.gizmo.setHover(mode);
      return;
    }

    // Drag handling
    if (!this.overlayStartQuat) {
      this.dragMode = null;
      return;
    }
    e.stopPropagation();

    if (this.dragAxisVec) {
      const axis = this.dragAxisVec;
      const pt = this.intersectRingPlane(
        e.clientX,
        e.clientY,
        axis,
      );
      if (pt && this.prevIntersect) {
        const a1 = this.angleAroundAxis(this.prevIntersect, axis);
        const a2 = this.angleAroundAxis(pt, axis);
        let delta = a2 - a1;
        if (delta > Math.PI) delta -= 2 * Math.PI;
        if (delta < -Math.PI) delta += 2 * Math.PI;
        if (this.dragMode !== "camera") delta = -delta;
        this.cumulativeAngle += delta;
        const rotQ = new THREE.Quaternion().setFromAxisAngle(
          axis,
          this.cumulativeAngle,
        );
        this.mesh.quaternion.copy(
          rotQ.multiply(this.overlayStartQuat),
        );
        this.prevIntersect.copy(pt);
      }
    }

    const q = this.mesh.quaternion;
    this.onOrientationChange([q.x, q.y, q.z, q.w]);
  }

  private onUp(_e: PointerEvent): void {
    this.dragMode = null;
    this.dragAxisVec = null;
    this.prevIntersect = null;
    this.cumulativeAngle = 0;
    this.controls.enabled = true;
  }

  dispose(): void {
    this.domElement.removeEventListener(
      "pointerdown",
      this.boundOnDown,
    );
    this.domElement.removeEventListener(
      "pointermove",
      this.boundOnMove,
    );
    this.domElement.removeEventListener(
      "pointerup",
      this.boundOnUp,
    );
  }
}
