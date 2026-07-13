import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { centroidTranslate, boundingRadius } from './centering';

type DragMode = null | 'axis-x' | 'axis-y' | 'axis-z' | 'camera';

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

  constructor(container: HTMLElement) {
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(0x2a2a2a);

    this.camera = new THREE.PerspectiveCamera(45, container.clientWidth / container.clientHeight, 0.1, 1000);
    this.camera.position.set(30, 20, 30);
    this.camera.lookAt(0, 0, 0);

    this.renderer = new THREE.WebGLRenderer({ antialias: true });
    this.renderer.setSize(container.clientWidth, container.clientHeight);
    this.renderer.setPixelRatio(window.devicePixelRatio);
    container.appendChild(this.renderer.domElement);

    this.controls = new OrbitControls(this.camera, this.renderer.domElement);
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
    window.addEventListener('resize', () => {
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
    this.billboardCameraRing();
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
    const theta = this.criticalAngleDeg * Math.PI / 180;
    const cosCrit = Math.cos(theta);
    const up = new THREE.Vector3(0, 1, 0);

    const triCount = this.faceNormals.length / 3;
    for (let t = 0; t < triCount; t++) {
      const fn = t * 3;
      const n = new THREE.Vector3(this.faceNormals[fn], this.faceNormals[fn + 1], this.faceNormals[fn + 2]);
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

  loadModel(positions: Float32Array, faceNormals?: Float32Array): void {
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
    this.destroyGizmo();

    const geometry = new THREE.BufferGeometry();
    geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    geometry.computeVertexNormals();

    const vertCount = positions.length / 3;
    geometry.setAttribute('color', this.makeColorAttr(vertCount));

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
    let cx = 0, cy = 0, cz = 0;
    for (let i = 0; i < vertCountLocal; i++) {
      cx += positions[i * 3];
      cy += positions[i * 3 + 1];
      cz += positions[i * 3 + 2];
    }
    cx /= vertCountLocal; cy /= vertCountLocal; cz /= vertCountLocal;
    const bake = centroidTranslate({ x: cx, y: cy, z: cz });

    this.boundingRadius = boundingRadius({ x: cx, y: cy, z: cz }, positions);
    geometry.translate(bake.x, bake.y, bake.z);
    this.modelGroup.position.set(0, this.boundingRadius, 0);

    geometry.computeBoundingBox();
    const bb = geometry.boundingBox!;
    const size = new THREE.Vector3();
    bb.getSize(size);
    const maxDim = Math.max(size.x, size.y, size.z);

    const dist = maxDim * 2.5;
    this.camera.position.set(dist * 0.8, dist * 0.6, dist * 0.8);
    this.controls.target.set(0, 0, 0);
    this.controls.update();

    if (this.faceNormals) this.colorOverhang();

    this.createGizmo();
    this.attachPointerHandlers();
  }

  showCandidate(quaternion: [number, number, number, number]): void {
    if (!this.mesh) return;
    this.mesh.quaternion.set(quaternion[0], quaternion[1], quaternion[2], quaternion[3]);
    if (this.faceNormals) this.colorOverhang();
  }

  applyYaw(yawDeg: number): void {
    if (!this.mesh) return;
    const baseQ = this.mesh.quaternion.clone();
    const yawQ = new THREE.Quaternion();
    yawQ.setFromAxisAngle(new THREE.Vector3(0, 1, 0), (yawDeg * Math.PI) / 180);
    this.mesh.quaternion.copy(yawQ.premultiply(baseQ));
    if (this.faceNormals) this.colorOverhang();
  }

  resetCamera(): void {
    if (!this.mesh) return;
    const bb = new THREE.Box3().setFromObject(this.mesh);
    const size = new THREE.Vector3();
    bb.getSize(size);
    const maxDim = Math.max(size.x, size.y, size.z);
    const dist = maxDim * 2.5;
    this.camera.position.set(dist * 0.8, dist * 0.6, dist * 0.8);
    this.controls.target.set(0, 0, 0);
    this.controls.update();
  }

  // ─── Gizmo ───────────────────────────────────────────────

  private gizmoGroup: THREE.Group | null = null;
  private gizmoRingX: THREE.Mesh | null = null;
  private gizmoRingY: THREE.Mesh | null = null;
  private gizmoRingZ: THREE.Mesh | null = null;
  private cameraRing: THREE.Mesh | null = null;
  private raycaster = new THREE.Raycaster();
  private hoveredRing: DragMode = null;
  private ringDefaultOpacities = new Map<THREE.Mesh, number>();

  private makeRing(radius: number, tube: number, color: number, opacity = 0.7): THREE.Mesh {
    const geometry = new THREE.TorusGeometry(radius, tube, 12, 48);
    const material = new THREE.MeshBasicMaterial({ color, transparent: true, opacity, depthWrite: false });
    const mesh = new THREE.Mesh(geometry, material);
    this.ringDefaultOpacities.set(mesh, opacity);
    return mesh;
  }

  private createGizmo(): void {
    this.destroyGizmo();
    this.gizmoGroup = new THREE.Group();
    this.gizmoGroup.position.copy(this.modelGroup.position);

    const r = this.boundingRadius * 1.3;
    const tube = Math.max(r * 0.006, 0.02);

    this.gizmoRingX = this.makeRing(r, tube, 0xff4444);
    this.gizmoRingX.rotation.y = Math.PI / 2;
    this.gizmoGroup.add(this.gizmoRingX);

    this.gizmoRingY = this.makeRing(r, tube, 0x44ff44);
    this.gizmoGroup.add(this.gizmoRingY);

    this.gizmoRingZ = this.makeRing(r, tube, 0x4488ff);
    this.gizmoRingZ.rotation.x = Math.PI / 2;
    this.gizmoGroup.add(this.gizmoRingZ);

    // Camera ring — outer, white, always faces camera via billboard
    const cr = this.boundingRadius * 1.6;
    const ct = Math.max(cr * 0.004, 0.015);
    this.cameraRing = this.makeRing(cr, ct, 0xcccccc, 0.35);
    this.gizmoGroup.add(this.cameraRing);

    this.scene.add(this.gizmoGroup);
  }

  private destroyGizmo(): void {
    if (this.gizmoGroup) {
      this.scene.remove(this.gizmoGroup);
      this.gizmoGroup.traverse((child) => {
        if (child instanceof THREE.Mesh) {
          child.geometry.dispose();
          (child.material as THREE.Material).dispose();
        }
      });
      this.gizmoGroup = null;
    }
    this.gizmoRingX = null;
    this.gizmoRingY = null;
    this.gizmoRingZ = null;
    this.cameraRing = null;
    this.ringDefaultOpacities.clear();
    this.hoveredRing = null;
  }

  private billboardCameraRing(): void {
    if (!this.cameraRing || !this.gizmoGroup) return;
    const worldPos = new THREE.Vector3();
    this.cameraRing.getWorldPosition(worldPos);
    const dir = new THREE.Vector3().subVectors(this.camera.position, worldPos).normalize();
    this.cameraRing.quaternion.setFromUnitVectors(new THREE.Vector3(0, 0, 1), dir);
  }

  // ─── Pointer Interaction ─────────────────────────────────

  private dragMode: DragMode = null;
  private overlayStartQuat: THREE.Quaternion | null = null;
  private dragAxisVec: THREE.Vector3 | null = null;
  private prevIntersect: THREE.Vector3 | null = null;
  private cumulativeAngle = 0;
  private handlersAttached = false;

  private onOrientationChange: ((q: [number, number, number, number]) => void) | null = null;

  private getNDC(clientX: number, clientY: number): THREE.Vector2 {
    const el = this.renderer.domElement;
    const rect = el.getBoundingClientRect();
    return new THREE.Vector2(
      ((clientX - rect.left) / rect.width) * 2 - 1,
      -((clientY - rect.top) / rect.height) * 2 + 1,
    );
  }

  private intersectRingPlane(clientX: number, clientY: number, axisVec: THREE.Vector3): THREE.Vector3 | null {
    const mouse = this.getNDC(clientX, clientY);
    this.raycaster.setFromCamera(mouse, this.camera);
    const center = new THREE.Vector3(0, this.boundingRadius, 0);
    const plane = new THREE.Plane().setFromNormalAndCoplanarPoint(axisVec, center);
    const pt = new THREE.Vector3();
    return this.raycaster.ray.intersectPlane(plane, pt) ? pt : null;
  }

  private angleAroundAxis(pt: THREE.Vector3, axis: THREE.Vector3): number {
    const center = new THREE.Vector3(0, this.boundingRadius, 0);
    const dir = new THREE.Vector3().subVectors(pt, center);
    const proj = dir.clone().sub(axis.clone().multiplyScalar(dir.dot(axis)));
    if (proj.lengthSq() < 0.0001) return 0;
    proj.normalize();
    const refBase = Math.abs(axis.y) > 0.9 ? new THREE.Vector3(1, 0, 0) :
                    Math.abs(axis.x) > 0.9 ? new THREE.Vector3(0, 0, 1) :
                    new THREE.Vector3(1, 0, 0);
    const ref = refBase.clone().sub(axis.clone().multiplyScalar(refBase.dot(axis)));
    ref.normalize();
    return Math.atan2(axis.dot(proj.clone().cross(ref)), proj.dot(ref));
  }

  private raycastAllRings(clientX: number, clientY: number): DragMode {
    if (!this.gizmoGroup) return null;
    const mouse = this.getNDC(clientX, clientY);
    this.raycaster.setFromCamera(mouse, this.camera);

    const meshes: { mesh: THREE.Mesh; mode: DragMode }[] = [];
    if (this.gizmoRingX) meshes.push({ mesh: this.gizmoRingX, mode: 'axis-x' });
    if (this.gizmoRingY) meshes.push({ mesh: this.gizmoRingY, mode: 'axis-y' });
    if (this.gizmoRingZ) meshes.push({ mesh: this.gizmoRingZ, mode: 'axis-z' });
    if (this.cameraRing) meshes.push({ mesh: this.cameraRing, mode: 'camera' });

    const hits = this.raycaster.intersectObjects(meshes.map(m => m.mesh));
    if (hits.length > 0) {
      for (const m of meshes) {
        if (hits[0].object === m.mesh) return m.mode;
      }
    }
    return null;
  }

  private getRingMesh(mode: DragMode): THREE.Mesh | null {
    switch (mode) {
      case 'axis-x': return this.gizmoRingX;
      case 'axis-y': return this.gizmoRingY;
      case 'axis-z': return this.gizmoRingZ;
      case 'camera': return this.cameraRing;
      default: return null;
    }
  }

  private attachPointerHandlers(): void {
    if (this.handlersAttached || !this.mesh) return;
    this.handlersAttached = true;
    const el = this.renderer.domElement;

    const onDown = (e: PointerEvent) => {
      if (!this.mesh) return;
      const mode = this.raycastAllRings(e.clientX, e.clientY);
      if (!mode) return;
      e.stopPropagation();
      this.dragMode = mode;
      this.overlayStartQuat = this.mesh.quaternion.clone();
      el.setPointerCapture(e.pointerId);
      this.controls.enabled = false;

      let axisVec: THREE.Vector3;
      if (mode === 'axis-x') {
        axisVec = new THREE.Vector3(1, 0, 0);
      } else if (mode === 'axis-y') {
        axisVec = new THREE.Vector3(0, 0, 1);
      } else if (mode === 'axis-z') {
        axisVec = new THREE.Vector3(0, 1, 0);
      } else {
        axisVec = new THREE.Vector3();
        this.camera.getWorldDirection(axisVec);
        axisVec.normalize();
      }
      this.dragAxisVec = axisVec;
      this.prevIntersect = this.intersectRingPlane(e.clientX, e.clientY, axisVec);
      this.cumulativeAngle = 0;
    };

    const onMove = (e: PointerEvent) => {
      // ─── Hover highlighting ────────────────────────────
      if (!this.dragMode && this.mesh) {
        const mode = this.raycastAllRings(e.clientX, e.clientY);
        if (mode !== this.hoveredRing) {
          // Restore previous ring's opacity
          if (this.hoveredRing) {
            const prev = this.getRingMesh(this.hoveredRing);
            if (prev) {
              const orig = this.ringDefaultOpacities.get(prev) ?? 0.7;
              (prev.material as THREE.MeshBasicMaterial).opacity = orig;
            }
          }
          // Highlight new ring
          if (mode) {
            const ring = this.getRingMesh(mode);
            if (ring) (ring.material as THREE.MeshBasicMaterial).opacity = 1.0;
          }
          this.hoveredRing = mode;
        }
        return;
      }

      // ─── Drag handling ─────────────────────────────────
      if (!this.dragMode || !this.overlayStartQuat || !this.mesh) {
        this.dragMode = null;
        return;
      }
      e.stopPropagation();

      if (this.dragAxisVec) {
        const axis = this.dragAxisVec;
        const pt = this.intersectRingPlane(e.clientX, e.clientY, axis);
        if (pt && this.prevIntersect) {
          const a1 = this.angleAroundAxis(this.prevIntersect, axis);
          const a2 = this.angleAroundAxis(pt, axis);
          let delta = a2 - a1;
          if (delta > Math.PI) delta -= 2 * Math.PI;
          if (delta < -Math.PI) delta += 2 * Math.PI;
          if (this.dragMode !== 'camera') delta = -delta;
          this.cumulativeAngle += delta;
          const rotQ = new THREE.Quaternion().setFromAxisAngle(axis, this.cumulativeAngle);
          this.mesh.quaternion.copy(rotQ.multiply(this.overlayStartQuat));
          this.prevIntersect.copy(pt);
        }
      }

      this.colorOverhang();
      if (this.onOrientationChange) {
        const q = this.mesh.quaternion;
        this.onOrientationChange([q.x, q.y, q.z, q.w]);
      }
    };

    const onUp = (e: PointerEvent) => {
      this.dragMode = null;
      this.dragAxisVec = null;
      this.prevIntersect = null;
      this.cumulativeAngle = 0;
      this.controls.enabled = true;
    };

    el.addEventListener('pointerdown', onDown, { capture: true });
    el.addEventListener('pointermove', onMove, { capture: true });
    el.addEventListener('pointerup', onUp, { capture: true });
  }

  // ─── Overlay Mode ───────────────────────────────────────

  private overlayActive = false;

  get isOverlayActive(): boolean {
    return this.overlayActive;
  }

  enterOverlayMode(onChange: (q: [number, number, number, number]) => void): void {
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
    this.renderer.dispose();
  }
}
