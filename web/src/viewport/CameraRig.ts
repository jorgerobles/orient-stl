import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { CAMERA_DIST_MULT } from "../constants";

export class CameraRig {
  constructor(
    private camera: THREE.PerspectiveCamera,
    private controls: OrbitControls,
  ) {}

  positionForBoundingBox(size: THREE.Vector3): void {
    const maxDim = Math.max(size.x, size.y, size.z);
    const dist = maxDim * CAMERA_DIST_MULT;
    this.camera.position.set(dist * 0.8, dist * 0.6, dist * 0.8);
    this.controls.target.set(0, 0, 0);
    this.controls.update();
  }

  reset(mesh: THREE.Mesh): void {
    const bb = new THREE.Box3().setFromObject(mesh);
    const size = new THREE.Vector3();
    bb.getSize(size);
    this.positionForBoundingBox(size);
  }
}
