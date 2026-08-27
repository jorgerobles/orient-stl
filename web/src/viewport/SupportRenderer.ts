import * as THREE from 'three';
import type { SupportResult, Support, RaftGeometry } from '../types';

const TYPE_COLORS: Record<string, number> = {
  light: 0x4caf50,
  medium: 0xffc107,
  heavy: 0xf44336,
};

const MIN_COLUMN_RADIUS = 0.8;
const BASE_RADIUS_FACTOR = 2.5;

export class SupportRenderer {
  private parent: THREE.Object3D;
  private group: THREE.Group;
  private columnMeshes: THREE.Mesh[] = [];
  private raftMesh: THREE.Mesh | null = null;
  private _visible = false;
  private offset: THREE.Vector3 = new THREE.Vector3();

  constructor(parent: THREE.Object3D) {
    this.parent = parent;
    this.group = new THREE.Group();
    this.group.name = 'supports';
    this.group.visible = false;
    this.parent.add(this.group);
  }

  setOffset(x: number, y: number, z: number): void {
    this.offset.set(x, y, z);
  }

  render(result: SupportResult): void {
    this.clear();

    console.log('[SupportRenderer] Rendering:', result.supports.length, 'supports,', result.islandCount, 'islands');
    if (result.supports.length > 0) {
      const s = result.supports[0];
      console.log('[SupportRenderer] Sample: type=' + s.contact.supportType + ' pos=' + JSON.stringify(s.contact.position) + ' base=' + JSON.stringify(s.contact.base) + ' tipDia=' + s.contact.tipDiameter);
    }

    this.renderRaft(result.raft);

    for (const support of result.supports) {
      this.renderColumn(support);
      this.renderDebugSphere(support);
    }
  }

  private renderDebugSphere(support: Support): void {
    const { contact } = support;
    const pos = new THREE.Vector3(...contact.position).add(this.offset);
    const base = new THREE.Vector3(...contact.base).add(this.offset);

    // Also log the mesh bounding box for comparison
    const parent = this.parent as any;
    const bbox = parent.children?.[0]?.geometry?.boundingBox;

    console.log('[DEBUG] pos=' + pos.toArray() + ' base=' + base.toArray() + ' offset=' + this.offset.toArray() + ' bbox=' + (bbox ? bbox.min.toArray() + '→' + bbox.max.toArray() : 'none'));

    // Large red sphere at contact point
    const tipGeo = new THREE.SphereGeometry(2, 16, 16);
    const tipMat = new THREE.MeshBasicMaterial({ color: 0xff0000 });
    const tipSphere = new THREE.Mesh(tipGeo, tipMat);
    tipSphere.position.copy(pos);
    this.group.add(tipSphere);
    this.columnMeshes.push(tipSphere);

    // Large blue sphere at base point
    const baseGeo = new THREE.SphereGeometry(2, 16, 16);
    const baseMat = new THREE.MeshBasicMaterial({ color: 0x0000ff });
    const baseSphere = new THREE.Mesh(baseGeo, baseMat);
    baseSphere.position.copy(base);
    this.group.add(baseSphere);
    this.columnMeshes.push(baseSphere);
  }

  private renderColumn(support: Support): void {
    const { contact } = support;
    const base = new THREE.Vector3(...contact.base).add(this.offset);
    const tip = new THREE.Vector3(...contact.position).add(this.offset);
    const height = base.distanceTo(tip);
    if (height < 0.01) return;

    const tipRadius = Math.max(contact.tipDiameter / 2, MIN_COLUMN_RADIUS);
    const baseRadius = tipRadius * BASE_RADIUS_FACTOR;

    const geo = new THREE.CylinderGeometry(tipRadius, baseRadius, height, 8);
    const color = TYPE_COLORS[contact.supportType] ?? 0x999999;
    const mat = new THREE.MeshPhongMaterial({
      color,
      transparent: true,
      opacity: 0.85,
      shininess: 30,
    });
    const mesh = new THREE.Mesh(geo, mat);

    const mid = base.clone().add(tip).multiplyScalar(0.5);
    mesh.position.copy(mid);

    const dir = tip.clone().sub(base).normalize();
    const up = new THREE.Vector3(0, 1, 0);
    const quat = new THREE.Quaternion().setFromUnitVectors(up, dir);
    mesh.quaternion.copy(quat);

    this.group.add(mesh);
    this.columnMeshes.push(mesh);
  }

  private renderRaft(raft: RaftGeometry): void {
    if (raft.vertices.length === 0) return;

    const geo = new THREE.BufferGeometry();
    const vertices = new Float32Array(raft.vertices);
    // Apply offset to raft vertices
    for (let i = 0; i < vertices.length; i += 3) {
      vertices[i] += this.offset.x;
      vertices[i + 1] += this.offset.y;
      vertices[i + 2] += this.offset.z;
    }
    geo.setAttribute('position', new THREE.Float32BufferAttribute(vertices, 3));
    geo.setIndex(new THREE.BufferAttribute(new Uint32Array(raft.triangles), 1));
    geo.computeVertexNormals();

    const mat = new THREE.MeshPhongMaterial({
      color: 0x9e9e9e,
      transparent: true,
      opacity: 0.4,
      side: THREE.DoubleSide,
      shininess: 10,
    });
    this.raftMesh = new THREE.Mesh(geo, mat);
    this.group.add(this.raftMesh);
  }

  clear(): void {
    for (const m of this.columnMeshes) {
      m.geometry.dispose();
      (m.material as THREE.Material).dispose();
      this.group.remove(m);
    }
    this.columnMeshes = [];

    if (this.raftMesh) {
      this.raftMesh.geometry.dispose();
      (this.raftMesh.material as THREE.Material).dispose();
      this.group.remove(this.raftMesh);
      this.raftMesh = null;
    }
  }

  setVisible(visible: boolean): void {
    this._visible = visible;
    this.group.visible = visible;
  }

  get isVisible(): boolean {
    return this._visible;
  }

  getColumnMeshes(): THREE.Mesh[] {
    return this.columnMeshes;
  }

  getRaftMesh(): THREE.Mesh | null {
    return this.raftMesh;
  }

  dispose(): void {
    this.clear();
    this.parent.remove(this.group);
  }
}
