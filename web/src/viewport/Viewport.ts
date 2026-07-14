import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { centroidTranslate, boundingRadius } from "../centering";
import { CAMERA_FOV, CAMERA_NEAR, CAMERA_FAR, INITIAL_CAMERA_POS } from "../constants";
import { GizmoController } from "./GizmoController";
import { CameraRig } from "./CameraRig";
import { DragHandler } from "./DragHandler";

export class Viewport {
  private scene: THREE.Scene;
  private camera: THREE.PerspectiveCamera;
  private renderer: THREE.WebGLRenderer;
  private controls: OrbitControls;
  private mesh: THREE.Mesh | null = null;
  private modelGroup: THREE.Group;
  private faceNormals: Float32Array | null = null;
  private criticalAngleDeg = 45;
  private plateGroup: THREE.Group;
  private boundingRadius = 0;
  private animationId = 0;
  private gizmo: GizmoController | null = null;
  private cameraRig: CameraRig | null = null;
  private dragHandler: DragHandler | null = null;

  constructor(container: HTMLElement) {
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0x2a2a2a);

    this.camera = new THREE.PerspectiveCamera(
      CAMERA_FOV,
      container.clientWidth / container.clientHeight,
      CAMERA_NEAR,
      CAMERA_FAR,
    );
    this.camera.position.set(
      INITIAL_CAMERA_POS[0],
      INITIAL_CAMERA_POS[1],
      INITIAL_CAMERA_POS[2],
    );
    this.camera.lookAt(0, 0, 0);

    this.renderer = new THREE.WebGLRenderer({ antialias: true });
    this.renderer.setSize(container.clientWidth, container.clientHeight);
    this.renderer.setPixelRatio(window.devicePixelRatio);
    container.appendChild(this.renderer.domElement);

    this.controls = new OrbitControls(
      this.camera,
      this.renderer.domElement,
    );
    this.controls.target.set(0, 0, 0);
    this.controls.update();

    this.modelGroup = new THREE.Group();
    this.scene.add(this.modelGroup);

    this.plateGroup = new THREE.Group();
    this.scene.add(this.plateGroup);

    this.addLights();
    this.addBuildPlate();
    this.resize();
    this.animate();
  }

  private addLights(): void {
    const ambient = new THREE.AmbientLight(0xffffff, 0.4);
    this.scene.add(ambient);
    const dir = new THREE.DirectionalLight(0xffffff, 0.8);
    dir.position.set(20, 30, 20);
    this.scene.add(dir);
    const fill = new THREE.DirectionalLight(0x8888ff, 0.3);
    fill.position.set(-20, 10, -20);
    this.scene.add(fill);
  }

  private addBuildPlate(): void {
    const grid = new THREE.GridHelper(60, 20, 0x888888, 0x555555);
    grid.position.y = 0;
    this.plateGroup.add(grid);

    const geo = new THREE.PlaneGeometry(60, 60);
    const mat = new THREE.MeshBasicMaterial({
      color: 0x446688,
      transparent: true,
      opacity: 0.08,
      side: THREE.DoubleSide,
      depthWrite: false,
    });
    const plane = new THREE.Mesh(geo, mat);
    plane.rotation.x = -Math.PI / 2;
    plane.position.y = 0;
    this.plateGroup.add(plane);
  }

  private resize(): void {
    window.addEventListener("resize", () => {
      const el = this.renderer.domElement.parentElement;
      if (!el) return;
      const w = el.clientWidth;
      const h = el.clientHeight;
      this.camera.aspect = w / h;
      this.camera.updateProjectionMatrix();
      this.renderer.setSize(w, h);
    });
  }

  private animate(): void {
    this.animationId = requestAnimationFrame(() => this.animate());
    this.controls.update();
    this.gizmo?.billboard(this.camera);
    this.renderer.render(this.scene, this.camera);
  }

  private makeColorAttr(vertexCount: number): THREE.BufferAttribute {
    const colors = new Float32Array(vertexCount * 3);
    return new THREE.BufferAttribute(colors, 3);
  }

  private colorOverhang(): void {
    if (!this.mesh || !this.faceNormals) return;
    const geom = this.mesh.geometry;
    const colors = geom.attributes.color as THREE.BufferAttribute;
    const q = this.mesh.quaternion;
    const theta = (this.criticalAngleDeg * Math.PI) / 180;
    const cosCrit = Math.cos(theta);
    const up = new THREE.Vector3(0, 1, 0);

    const triCount = this.faceNormals.length / 3;
    for (let t = 0; t < triCount; t++) {
      const fn = t * 3;
      const n = new THREE.Vector3(
        this.faceNormals[fn],
        this.faceNormals[fn + 1],
        this.faceNormals[fn + 2],
      );
      n.applyQuaternion(q);
      const isOverhang = n.dot(up) < -cosCrit;
      const r = isOverhang ? 1 : 0.3;
      const g = isOverhang ? 0.15 : 0.7;
      const b = isOverhang ? 0.15 : 1;
      const vi = t * 3;
      colors.setXYZ(vi, r, g, b);
      colors.setXYZ(vi + 1, r, g, b);
      colors.setXYZ(vi + 2, r, g, b);
    }
    colors.needsUpdate = true;
  }

  setCriticalAngle(deg: number): void {
    this.criticalAngleDeg = deg;
    this.colorOverhang();
  }

  loadModel(
    positions: Float32Array,
    faceNormals?: Float32Array,
  ): void {
    while (this.modelGroup.children.length > 0) {
      const child = this.modelGroup.children[0];
      this.modelGroup.remove(child);
      if (child instanceof THREE.Mesh) {
        child.geometry.dispose();
        (child.material as THREE.Material).dispose();
      }
    }
    this.mesh = null;
    this.faceNormals = faceNormals || null;
    if (this.dragHandler) {
      this.dragHandler.dispose();
      this.dragHandler = null;
    }
    if (this.gizmo) {
      this.scene.remove(this.gizmo.group);
      this.gizmo.dispose();
      this.gizmo = null;
    }

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute(
      "position",
      new THREE.BufferAttribute(positions, 3),
    );
    geometry.computeVertexNormals();

    const vertCount = positions.length / 3;
    geometry.setAttribute("color", this.makeColorAttr(vertCount));

    const material = new THREE.MeshStandardMaterial({
      color: 0xffffff,
      flatShading: true,
      side: THREE.DoubleSide,
      metalness: 0.1,
      roughness: 0.6,
      vertexColors: true,
    });
    this.mesh = new THREE.Mesh(geometry, material);
    this.modelGroup.add(this.mesh);

    const vertCountLocal = positions.length / 3;
    let cx = 0,
      cy = 0,
      cz = 0;
    for (let i = 0; i < vertCountLocal; i++) {
      cx += positions[i * 3];
      cy += positions[i * 3 + 1];
      cz += positions[i * 3 + 2];
    }
    cx /= vertCountLocal;
    cy /= vertCountLocal;
    cz /= vertCountLocal;
    const bake = centroidTranslate({ x: cx, y: cy, z: cz });

    this.boundingRadius = boundingRadius(
      { x: cx, y: cy, z: cz },
      positions,
    );
    geometry.translate(bake.x, bake.y, bake.z);
    this.modelGroup.position.set(0, this.boundingRadius, 0);

    geometry.computeBoundingBox();
    const bb = geometry.boundingBox!;
    const size = new THREE.Vector3();
    bb.getSize(size);

    this.cameraRig = new CameraRig(this.camera, this.controls);
    this.cameraRig.positionForBoundingBox(size);

    if (this.faceNormals) this.colorOverhang();

    this.gizmo = new GizmoController(
      new THREE.Vector3(0, this.boundingRadius),
      this.boundingRadius,
    );
    this.scene.add(this.gizmo.group);

    this.dragHandler = new DragHandler(
      this.gizmo,
      this.mesh,
      this.camera,
      this.renderer.domElement,
      this.boundingRadius,
      this.controls,
      (q) => {
        this.colorOverhang();
        if (this.onOrientationChange) this.onOrientationChange(q);
      },
    );
  }

  showCandidate(quaternion: [number, number, number, number]): void {
    if (!this.mesh) return;
    this.mesh.quaternion.set(
      quaternion[0],
      quaternion[1],
      quaternion[2],
      quaternion[3],
    );
    if (this.faceNormals) this.colorOverhang();
  }

  applyYaw(yawDeg: number): void {
    if (!this.mesh) return;
    const baseQ = this.mesh.quaternion.clone();
    const yawQ = new THREE.Quaternion();
    yawQ.setFromAxisAngle(
      new THREE.Vector3(0, 1, 0),
      (yawDeg * Math.PI) / 180,
    );
    this.mesh.quaternion.copy(yawQ.premultiply(baseQ));
    if (this.faceNormals) this.colorOverhang();
  }

  resetCamera(): void {
    if (!this.mesh || !this.cameraRig) return;
    this.cameraRig.reset(this.mesh);
  }

  // ─── Overlay Mode ───────────────────────────────────────

  private overlayActive = false;
  private onOrientationChange:
    | ((q: [number, number, number, number]) => void)
    | null = null;

  get isOverlayActive(): boolean {
    return this.overlayActive;
  }

  enterOverlayMode(
    onChange: (q: [number, number, number, number]) => void,
  ): void {
    if (!this.mesh) return;
    this.overlayActive = true;
    this.onOrientationChange = onChange;
  }

  exitOverlayMode(): void {
    this.overlayActive = false;
    this.onOrientationChange = null;
  }

  getMeshQuaternion(): [number, number, number, number] {
    if (!this.mesh) return [1, 0, 0, 0];
    const q = this.mesh.quaternion;
    return [q.x, q.y, q.z, q.w];
  }

  dispose(): void {
    cancelAnimationFrame(this.animationId);
    this.gizmo?.dispose();
    this.dragHandler?.dispose();
    this.renderer.dispose();
  }
}
