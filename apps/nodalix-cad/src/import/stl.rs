use crate::{
    document::{Document, MeshReference},
    geometry::Point3,
    mesh::mesh::{Mesh, MeshTransform, Triangle},
};
use std::{fs, path::Path};

use super::ImportSummary;

pub fn import_stl(path: &Path, document: &mut Document) -> Result<ImportSummary, String> {
    let bytes = fs::read(path).map_err(|err| err.to_string())?;
    let mesh = if looks_ascii_stl(&bytes) {
        parse_ascii_stl(&String::from_utf8_lossy(&bytes))?
    } else {
        parse_binary_stl(&bytes)?
    };
    let bbox = mesh.bounding_box;
    let dimensions = crate::mesh::analysis::dimensions_label(bbox);
    let triangle_count = mesh.triangles.len();
    document.mesh_references.push(MeshReference {
        path: path.display().to_string(),
        format: "STL".to_string(),
        triangle_count,
        bounding_box: bbox,
        transform: MeshTransform::default(),
        warnings: Vec::new(),
    });
    document.modified = true;

    Ok(ImportSummary {
        file_name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("mesh.stl")
            .to_string(),
        format: "STL mesh".to_string(),
        summary: "STL mesh imported as a mesh reference with metadata and bounding box."
            .to_string(),
        details: vec![
            ("Triangles".to_string(), triangle_count.to_string()),
            ("Dimensions".to_string(), dimensions),
        ],
        warnings: Vec::new(),
    })
}

fn looks_ascii_stl(bytes: &[u8]) -> bool {
    bytes.starts_with(b"solid")
        && String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).contains("facet")
}

fn parse_ascii_stl(data: &str) -> Result<Mesh, String> {
    let mut vertices = Vec::new();
    for line in data.lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() == 4 && parts[0].eq_ignore_ascii_case("vertex") {
            let x = parts[1].parse::<f64>().map_err(|err| err.to_string())?;
            let y = parts[2].parse::<f64>().map_err(|err| err.to_string())?;
            let z = parts[3].parse::<f64>().map_err(|err| err.to_string())?;
            vertices.push(Point3 { x, y, z });
        }
    }
    let triangles = vertices
        .chunks_exact(3)
        .map(|chunk| Triangle {
            vertices: [chunk[0], chunk[1], chunk[2]],
        })
        .collect::<Vec<_>>();
    if triangles.is_empty() {
        return Err("ASCII STL did not contain triangles".to_string());
    }
    Ok(Mesh::from_triangles(triangles))
}

fn parse_binary_stl(bytes: &[u8]) -> Result<Mesh, String> {
    if bytes.len() < 84 {
        return Err("Binary STL is too small".to_string());
    }
    let count =
        u32::from_le_bytes(bytes[80..84].try_into().map_err(|_| "Invalid STL header")?) as usize;
    let expected = 84usize.saturating_add(count.saturating_mul(50));
    if bytes.len() < expected {
        return Err("Binary STL is truncated".to_string());
    }
    let mut triangles = Vec::with_capacity(count);
    let mut offset = 84;
    for _ in 0..count {
        offset += 12;
        let mut verts = [Point3::default(); 3];
        for vertex in &mut verts {
            let x = f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as f64;
            let y = f32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as f64;
            let z = f32::from_le_bytes(bytes[offset + 8..offset + 12].try_into().unwrap()) as f64;
            *vertex = Point3 { x, y, z };
            offset += 12;
        }
        offset += 2;
        triangles.push(Triangle { vertices: verts });
    }
    Ok(Mesh::from_triangles(triangles))
}
