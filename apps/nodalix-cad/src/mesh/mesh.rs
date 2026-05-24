use crate::geometry::{BoundingBox3, Point3};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Mesh {
    pub triangles: Vec<Triangle>,
    pub bounding_box: Option<BoundingBox3>,
}

#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    pub vertices: [Point3; 3],
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct MeshTransform {
    pub scale: f64,
    pub translation: Point3,
    pub rotation_deg: Point3,
}

impl Default for MeshTransform {
    fn default() -> Self {
        Self {
            scale: 1.0,
            translation: Point3::default(),
            rotation_deg: Point3::default(),
        }
    }
}

impl Mesh {
    pub fn from_triangles(triangles: Vec<Triangle>) -> Self {
        let mut bbox = BoundingBox3::empty();
        for triangle in &triangles {
            for vertex in triangle.vertices {
                bbox.include(vertex);
            }
        }
        Self {
            triangles,
            bounding_box: bbox.is_valid().then_some(bbox),
        }
    }
}
