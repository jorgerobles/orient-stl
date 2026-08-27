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
  private scene: THREE.Scene;
  private group: THREE.Group;
  private columnMeshes: THREE.Mesh[] = [];
  private raftMesh: THREE.Mesh | null = null;
  private _visible = false;

  constructor(scene: THREE.Scene) {
    this.scene = scene;
    this.group = new THREE.Group();
    this.group.name = 'supports';
    this.group.visible = false;
    this.scene.add(this.group);
  }

  render(result: SupportResult): void {
    this.clear();
    this.renderRaft(result.raft);

    for (const support of result.supports) {
      this.renderColumn(support);
    }
  }

  private renderColumn(support: Support): void {
    const { contact } = support;
    const base = new THREE.Vector3(...contact.base);
    const tip = new THREE.Vector3(...contact.position);
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
    geo.setAttribute('position', new THREE.Float32BufferAttribute(raft.vertices, 3));
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
    this.scene.remove(this.group);
  }
}
