mod mesh;
mod repair;
mod analysis;
mod types;

pub use types::*;
pub use mesh::MeshData;
pub use repair::*;
pub use analysis::*;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct GeometryKernel {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    normals: Vec<f32>,
    bounding_box: Option<BBox>,
}

#[wasm_bindgen]
impl GeometryKernel {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            bounding_box: None,
        }
    }

    #[wasm_bindgen(js_name = import)]
    pub fn import(
        &mut self,
        positions: &[f32],
        face_indices: &[u32],
        center: bool,
    ) -> JsValue {
        self.vertices = positions.to_vec();
        self.indices = face_indices.to_vec();
        
        if center {
            self.center_mesh();
        }
        
        self.compute_normals();
        self.compute_bounding_box();
        
        let result = ImportResult {
            bounding_box: self.bounding_box.clone().unwrap_or_default(),
            vertex_count: self.vertices.len() / 3,
            face_count: self.indices.len() / 3,
        };
        
        serde_wasm_bindgen::to_value(&result).unwrap_or_default()
    }

    #[wasm_bindgen(js_name = importRaw)]
    pub fn import_raw(
        &mut self,
        positions: &[f32],
        center: bool,
    ) -> JsValue {
        self.vertices = positions.to_vec();
        
        // Generate indices for non-indexed geometry
        self.indices = (0..(self.vertices.len() / 3) as u32).collect();
        
        if center {
            self.center_mesh();
        }
        
        self.compute_normals();
        self.compute_bounding_box();
        
        let result = ImportResult {
            bounding_box: self.bounding_box.clone().unwrap_or_default(),
            vertex_count: self.vertices.len() / 3,
            face_count: self.indices.len() / 3,
        };
        
        serde_wasm_bindgen::to_value(&result).unwrap_or_default()
    }

    #[wasm_bindgen(js_name = fillHole)]
    pub fn fill_hole(&mut self, options: JsValue) -> JsValue {
        let opts: RepairOptions = serde_wasm_bindgen::from_value(options)
            .unwrap_or_default();
        
        let mut result = RepairResult::default();
        
        if opts.reverse_misoriented_surfaces {
            result.flips += repair::fix_normals(&mut self.vertices, &mut self.indices);
        }
        
        if opts.delete_isolated_surfaces {
            let removed = repair::remove_isolated_surfaces(
                &mut self.vertices,
                &mut self.indices,
                opts.isolated_surface_threshold,
            );
            result.removed_triangles += removed;
        }
        
        if opts.delete_invisible_surfaces {
            let removed = repair::remove_invisible_surfaces(&mut self.vertices, &mut self.indices);
            result.removed_triangles += removed;
        }
        
        if opts.fill_holes {
            let mut total_filled = 0;
            for _ in 0..opts.max_passes {
                let filled = repair::fill_boundary_holes(
                    &mut self.vertices,
                    &mut self.indices,
                    opts.max_groups,
                );
                total_filled += filled;
                if filled == 0 {
                    break;
                }
            }
            result.filled_holes = total_filled;
        }
        
        self.compute_normals();
        self.compute_bounding_box();
        
        result.vertex_count = self.vertices.len() / 3;
        result.face_count = self.indices.len() / 3;
        
        serde_wasm_bindgen::to_value(&result).unwrap_or_default()
    }

    #[wasm_bindgen(js_name = analyze)]
    pub fn analyze(&self) -> JsValue {
        let analysis = analysis::analyze_mesh(&self.vertices, &self.indices);
        serde_wasm_bindgen::to_value(&analysis).unwrap_or_default()
    }

    #[wasm_bindgen(js_name = getVertices)]
    pub fn get_vertices(&self) -> Vec<f32> {
        self.vertices.clone()
    }

    #[wasm_bindgen(js_name = getIndices)]
    pub fn get_indices(&self) -> Vec<u32> {
        self.indices.clone()
    }

    #[wasm_bindgen(js_name = getNormals)]
    pub fn get_normals(&self) -> Vec<f32> {
        self.normals.clone()
    }

    #[wasm_bindgen(js_name = getBoundingBox)]
    pub fn get_bounding_box(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.bounding_box).unwrap_or_default()
    }

    fn center_mesh(&mut self) {
        if self.vertices.is_empty() {
            return;
        }
        
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        
        for v in self.vertices.chunks(3) {
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }
        
        let center = [
            (min[0] + max[0]) / 2.0,
            (min[1] + max[1]) / 2.0,
            (min[2] + max[2]) / 2.0,
        ];
        
        for v in self.vertices.chunks_mut(3) {
            v[0] -= center[0];
            v[1] -= center[1];
            v[2] -= center[2];
        }
    }

    fn compute_normals(&mut self) {
        self.normals = vec![0.0; self.vertices.len()];
        
        for tri in self.indices.chunks(3) {
            if tri.len() < 3 {
                continue;
            }
            
            let i0 = tri[0] as usize * 3;
            let i1 = tri[1] as usize * 3;
            let i2 = tri[2] as usize * 3;
            
            let v0 = [self.vertices[i0], self.vertices[i0 + 1], self.vertices[i0 + 2]];
            let v1 = [self.vertices[i1], self.vertices[i1 + 1], self.vertices[i1 + 2]];
            let v2 = [self.vertices[i2], self.vertices[i2 + 1], self.vertices[i2 + 2]];
            
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            
            let normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];
            
            let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            if len > 0.0 {
                let normal = [normal[0] / len, normal[1] / len, normal[2] / len];
                for &idx in tri.iter() {
                    let i = idx as usize * 3;
                    self.normals[i] += normal[0];
                    self.normals[i + 1] += normal[1];
                    self.normals[i + 2] += normal[2];
                }
            }
        }
        
        for n in self.normals.chunks_mut(3) {
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            if len > 0.0 {
                n[0] /= len;
                n[1] /= len;
                n[2] /= len;
            }
        }
    }

    fn compute_bounding_box(&mut self) {
        if self.vertices.is_empty() {
            self.bounding_box = None;
            return;
        }
        
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        
        for v in self.vertices.chunks(3) {
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }
        
        self.bounding_box = Some(BBox {
            min: Vec3 { x: min[0], y: min[1], z: min[2] },
            max: Vec3 { x: max[0], y: max[1], z: max[2] },
        });
    }
}

impl Default for GeometryKernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::MeshData;

    #[test]
    fn test_kernel_internal_state() {
        let mut kernel = GeometryKernel::new();
        
        let positions = vec![
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
        ];
        let indices = vec![0, 1, 2];
        
        kernel.vertices = positions;
        kernel.indices = indices;
        kernel.compute_normals();
        kernel.compute_bounding_box();
        
        assert_eq!(kernel.vertices.len(), 9);
        assert_eq!(kernel.indices.len(), 3);
        assert_eq!(kernel.normals.len(), 9);
        assert!(kernel.bounding_box.is_some());
        
        let bb = kernel.bounding_box.as_ref().unwrap();
        assert_eq!(bb.min.x, 0.0);
        assert_eq!(bb.max.y, 1.0);
    }

    #[test]
    fn test_fix_normals_inconsistent() {
        let mut vertices = vec![
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        // Two triangles sharing edge (0,1) traversed in same direction.
        // This means one triangle has inconsistent winding.
        let mut indices = vec![
            0, 1, 2,
            0, 1, 3,
        ];
        
        let flips = repair::fix_normals(&mut vertices, &mut indices);
        assert_eq!(flips, 1);
    }

    #[test]
    fn test_fix_normals_consistent() {
        let mut vertices = vec![
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,
        ];
        // Two triangles sharing edge (0,1) traversed in opposite directions.
        let mut indices = vec![
            0, 1, 2,
            1, 0, 3,
        ];
        
        let flips = repair::fix_normals(&mut vertices, &mut indices);
        assert_eq!(flips, 0);
    }

    #[test]
    fn test_remove_invisible_surfaces() {
        let mut vertices = vec![
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            2.0, 0.0, 0.0,
            2.0, 0.0, 0.0,
            2.0, 1.0, 0.0,
        ];
        let mut indices = vec![
            0, 1, 2,
            3, 4, 5,
        ];
        
        let removed = repair::remove_invisible_surfaces(&mut vertices, &mut indices);
        
        assert_eq!(removed, 1);
        assert_eq!(indices.len(), 3);
    }

    #[test]
    fn test_analyze_mesh() {
        let vertices = vec![
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
        ];
        let indices = vec![0, 1, 2];
        
        let analysis = analysis::analyze_mesh(&vertices, &indices);
        
        assert_eq!(analysis.vertex_count, 3);
        assert_eq!(analysis.face_count, 1);
        assert!(analysis.boundary_edges > 0);
        assert!(!analysis.is_watertight);
    }

    #[test]
    fn test_analyze_watertight_cube() {
        let vertices = vec![
            0.0, 0.0, 0.0,  1.0, 0.0, 0.0,  1.0, 1.0, 0.0,  0.0, 1.0, 0.0,
            0.0, 0.0, 1.0,  1.0, 0.0, 1.0,  1.0, 1.0, 1.0,  0.0, 1.0, 1.0,
        ];
        // 12 triangles forming a cube
        let indices = vec![
            0,1,2, 0,2,3,  4,6,5, 4,7,6,
            0,4,5, 0,5,1,  2,6,7, 2,7,3,
            0,3,7, 0,7,4,  1,5,6, 1,6,2,
        ];
        
        let analysis = analysis::analyze_mesh(&vertices, &indices);
        
        assert_eq!(analysis.vertex_count, 8);
        assert_eq!(analysis.face_count, 12);
        assert_eq!(analysis.boundary_edges, 0);
        assert!(analysis.is_watertight);
    }

    #[test]
    fn test_fill_boundary_holes() {
        let mut vertices = vec![
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            1.0, 1.0, 0.0,
            0.0, 1.0, 0.0,
        ];
        // Two triangles forming a quad with one triangle missing
        let mut indices = vec![
            0, 1, 2,
        ];
        
        let filled = repair::fill_boundary_holes(&mut vertices, &mut indices, 512);
        
        assert!(filled > 0);
        assert!(indices.len() > 3);
    }

    #[test]
    fn test_mesh_data_from_raw() {
        let positions = vec![
            0.0, 0.0, 0.0,
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
        ];
        let indices = vec![0, 1, 2];
        
        let mesh = MeshData::from_raw(&positions, &indices);
        
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.indices.len(), 1);
        assert_eq!(mesh.normals.len(), 1);
        assert_eq!(mesh.areas.len(), 1);
    }

    #[test]
    fn test_bounding_box() {
        let vertices = vec![
            -1.0, -2.0, -3.0,
            1.0, 2.0, 3.0,
        ];
        let indices = vec![0, 0, 1];
        
        let mesh = MeshData::from_raw(&vertices, &indices);
        let bbox = mesh.bounding_box().unwrap();
        
        assert_eq!(bbox.min.x, -1.0);
        assert_eq!(bbox.min.y, -2.0);
        assert_eq!(bbox.min.z, -3.0);
        assert_eq!(bbox.max.x, 1.0);
        assert_eq!(bbox.max.y, 2.0);
        assert_eq!(bbox.max.z, 3.0);
    }
}
