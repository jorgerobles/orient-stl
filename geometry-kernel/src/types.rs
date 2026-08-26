use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<[f32; 3]> for Vec3 {
    fn from(arr: [f32; 3]) -> Self {
        Self { x: arr[0], y: arr[1], z: arr[2] }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct BBox {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportResult {
    pub bounding_box: BBox,
    pub vertex_count: usize,
    pub face_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct RepairOptions {
    pub delete_isolated_surfaces: bool,
    pub isolated_surface_threshold: f32,
    pub reverse_misoriented_surfaces: bool,
    pub delete_invisible_surfaces: bool,
    pub fill_holes: bool,
    pub max_passes: u32,
    pub max_groups: u32,
}

impl Default for RepairOptions {
    fn default() -> Self {
        Self {
            delete_isolated_surfaces: false,
            isolated_surface_threshold: 1.5,
            reverse_misoriented_surfaces: true,
            delete_invisible_surfaces: true,
            fill_holes: true,
            max_passes: 16,
            max_groups: 512,
        }
    }
}

#[wasm_bindgen]
impl RepairOptions {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
    
    #[wasm_bindgen(js_name = setFillHoles)]
    pub fn set_fill_holes(&mut self, value: bool) {
        self.fill_holes = value;
    }
    
    #[wasm_bindgen(js_name = setMaxPasses)]
    pub fn set_max_passes(&mut self, value: u32) {
        self.max_passes = value;
    }
    
    #[wasm_bindgen(js_name = setReverseNormals)]
    pub fn set_reverse_normals(&mut self, value: bool) {
        self.reverse_misoriented_surfaces = value;
    }
    
    #[wasm_bindgen(js_name = setDeleteIsolated)]
    pub fn set_delete_isolated(&mut self, value: bool) {
        self.delete_isolated_surfaces = value;
    }
    
    #[wasm_bindgen(js_name = setIsolatedThreshold)]
    pub fn set_isolated_threshold(&mut self, value: f32) {
        self.isolated_surface_threshold = value;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepairResult {
    pub filled_holes: u32,
    pub flips: u32,
    pub removed_triangles: u32,
    pub vertex_count: usize,
    pub face_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeshAnalysis {
    pub vertex_count: usize,
    pub face_count: usize,
    pub boundary_edges: u32,
    pub genus: i32,
    pub is_watertight: bool,
    pub surface_area: f32,
    pub volume: f32,
}
