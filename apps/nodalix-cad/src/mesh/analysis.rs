use crate::geometry::BoundingBox3;

pub fn dimensions_label(bbox: Option<BoundingBox3>) -> String {
    match bbox {
        Some(bbox) => {
            let size = bbox.size();
            format!("{:.3} x {:.3} x {:.3}", size.x, size.y, size.z)
        }
        None => "unknown".to_string(),
    }
}
