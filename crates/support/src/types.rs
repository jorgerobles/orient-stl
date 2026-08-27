use serde::{Deserialize, Serialize};

/// Configuration for support generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportConfig {
    /// Layer height in mm (default 0.05mm).
    pub layer_height: f32,
    /// Volume threshold for Light support in mm³ (default 50.0).
    pub light_threshold: f32,
    /// Volume threshold for Medium support in mm³ (default 500.0).
    pub medium_threshold: f32,
    /// Tip diameter for Light supports in mm (default 0.25mm).
    pub light_tip_diameter: f32,
    /// Tip diameter for Medium supports in mm (default 0.40mm).
    pub medium_tip_diameter: f32,
    /// Tip diameter for Heavy supports in mm (default 0.80mm).
    pub heavy_tip_diameter: f32,
    /// Penetration depth for Light supports in mm (default 0.2mm).
    pub light_penetration: f32,
    /// Penetration depth for Medium supports in mm (default 0.3mm).
    pub medium_penetration: f32,
    /// Penetration depth for Heavy supports in mm (default 0.4mm).
    pub heavy_penetration: f32,
    /// Contact point spacing range for Light supports in mm (min, max).
    pub light_spacing: (f32, f32),
    /// Contact point spacing range for Medium supports in mm (min, max).
    pub medium_spacing: (f32, f32),
    /// Contact point spacing range for Heavy supports in mm (min, max).
    pub heavy_spacing: (f32, f32),
    /// Raft thickness in mm (default 1.0mm).
    pub raft_thickness: f32,
    /// Raft line width in mm (default 1.5mm).
    pub raft_line_width: f32,
    /// Rasterization grid cell size in mm (default 0.5mm).
    pub cell_size: f32,
}

impl Default for SupportConfig {
    fn default() -> Self {
        Self {
            layer_height: 0.05,
            light_threshold: 50.0,
            medium_threshold: 500.0,
            light_tip_diameter: 0.25,
            medium_tip_diameter: 0.40,
            heavy_tip_diameter: 0.80,
            light_penetration: 0.2,
            medium_penetration: 0.3,
            heavy_penetration: 0.4,
            light_spacing: (2.5, 6.0),
            medium_spacing: (2.0, 5.0),
            heavy_spacing: (1.5, 3.5),
            raft_thickness: 1.0,
            raft_line_width: 1.5,
            cell_size: 0.5,
        }
    }
}

/// Classification of support type based on volume above.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SupportType {
    Light,
    Medium,
    Heavy,
}

/// A disconnected overhang region detected by island detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Island {
    /// Grid coordinates of island pixels (cell_x, cell_y).
    pub pixels: Vec<(u32, u32)>,
    /// 2D centroid of the island in world units.
    pub centroid: [f32; 2],
    /// Estimated area of the island in mm².
    pub area: f32,
    /// Lowest layer z where this island appears.
    pub z_min: f32,
    /// Highest layer z where this island appears.
    pub z_max: f32,
}

/// A contact point where a support column meets the overhang.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactPoint {
    /// 3D contact point on the island surface.
    pub position: [f32; 3],
    /// 3D base point on the raft plane.
    pub base: [f32; 3],
    /// Support type for this contact.
    pub support_type: SupportType,
    /// Tip diameter in mm.
    pub tip_diameter: f32,
    /// Penetration depth in mm.
    pub penetration: f32,
}

/// A single support column with volume information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Support {
    /// Contact point details.
    pub contact: ContactPoint,
    /// Volume of material above this contact point in mm³.
    pub volume_above: f32,
}

/// Geometry of the raft (base plate connecting all supports).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RaftGeometry {
    /// Flat vertex positions [x,y,z, x,y,z, ...].
    pub vertices: Vec<f32>,
    /// Triangle indices.
    pub triangles: Vec<u32>,
    /// Line connection indices (pairs of indices for line segments).
    pub lines: Vec<u32>,
}

/// Complete result of support generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupportResult {
    /// Generated supports.
    pub supports: Vec<Support>,
    /// Raft geometry connecting all supports.
    pub raft: RaftGeometry,
    /// Total support volume in mm³.
    pub total_volume: f32,
    /// Number of islands detected.
    pub island_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_values() {
        let config = SupportConfig::default();
        assert!(config.layer_height > 0.0);
        assert!(config.layer_height < 1.0);
        assert!(config.light_threshold < config.medium_threshold);
        assert!(config.light_tip_diameter < config.medium_tip_diameter);
        assert!(config.medium_tip_diameter < config.heavy_tip_diameter);
        assert!(config.light_spacing.0 < config.light_spacing.1);
        assert!(config.medium_spacing.0 < config.medium_spacing.1);
        assert!(config.heavy_spacing.0 < config.heavy_spacing.1);
        assert!(config.raft_thickness > 0.0);
        assert!(config.cell_size > 0.0);
    }

    #[test]
    fn support_type_ordering() {
        assert_ne!(SupportType::Light, SupportType::Medium);
        assert_ne!(SupportType::Medium, SupportType::Heavy);
        assert_ne!(SupportType::Light, SupportType::Heavy);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = SupportConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: SupportConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.layer_height, parsed.layer_height);
        assert_eq!(config.medium_threshold, parsed.medium_threshold);
    }
}
