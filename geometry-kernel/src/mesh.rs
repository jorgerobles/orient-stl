use crate::types::BBox;

#[derive(Debug, Clone)]
pub struct MeshData {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<[u32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub areas: Vec<f32>,
}

impl MeshData {
    pub fn from_raw(positions: &[f32], face_indices: &[u32]) -> Self {
        let vertices: Vec<[f32; 3]> = positions
            .chunks(3)
            .filter(|c| c.len() == 3)
            .map(|c| [c[0], c[1], c[2]])
            .collect();
        
        let indices: Vec<[u32; 3]> = face_indices
            .chunks(3)
            .filter(|c| c.len() == 3)
            .map(|c| [c[0], c[1], c[2]])
            .collect();
        
        let mut mesh = Self {
            vertices,
            indices,
            normals: Vec::new(),
            areas: Vec::new(),
        };
        
        mesh.compute_normals();
        mesh.compute_areas();
        
        mesh
    }
    
    pub fn compute_normals(&mut self) {
        self.normals = self.indices
            .iter()
            .map(|tri| {
                let v0 = self.vertices[tri[0] as usize];
                let v1 = self.vertices[tri[1] as usize];
                let v2 = self.vertices[tri[2] as usize];
                
                let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
                let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
                
                let normal = [
                    edge1[1] * edge2[2] - edge1[2] * edge2[1],
                    edge1[2] * edge2[0] - edge1[0] * edge2[2],
                    edge1[0] * edge2[1] - edge1[1] * edge2[0],
                ];
                
                let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
                if len > 0.0 {
                    [normal[0] / len, normal[1] / len, normal[2] / len]
                } else {
                    [0.0, 0.0, 1.0]
                }
            })
            .collect();
    }
    
    pub fn compute_areas(&mut self) {
        self.areas = self.indices
            .iter()
            .map(|tri| {
                let v0 = self.vertices[tri[0] as usize];
                let v1 = self.vertices[tri[1] as usize];
                let v2 = self.vertices[tri[2] as usize];
                
                let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
                let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
                
                let cross = [
                    edge1[1] * edge2[2] - edge1[2] * edge2[1],
                    edge1[2] * edge2[0] - edge1[0] * edge2[2],
                    edge1[0] * edge2[1] - edge1[1] * edge2[0],
                ];
                
                let len = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
                len * 0.5
            })
            .collect();
    }
    
    pub fn bounding_box(&self) -> Option<BBox> {
        if self.vertices.is_empty() {
            return None;
        }
        
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        
        for v in &self.vertices {
            for i in 0..3 {
                min[i] = min[i].min(v[i]);
                max[i] = max[i].max(v[i]);
            }
        }
        
        Some(BBox {
            min: crate::types::Vec3 { x: min[0], y: min[1], z: min[2] },
            max: crate::types::Vec3 { x: max[0], y: max[1], z: max[2] },
        })
    }
    
    pub fn total_area(&self) -> f32 {
        self.areas.iter().sum()
    }
}
