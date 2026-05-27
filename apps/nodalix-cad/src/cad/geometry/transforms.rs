use super::{point::Point2, vector::Vector2};
use serde::{Deserialize, Serialize};

/// 2D affine transform: rotation + uniform scale + translation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform2 {
    pub translation: Vector2,
    pub rotation: f64,
    pub scale: f64,
}

impl Default for Transform2 {
    fn default() -> Self {
        Self {
            translation: Vector2::ZERO,
            rotation: 0.0,
            scale: 1.0,
        }
    }
}

impl Transform2 {
    pub fn identity() -> Self {
        Self::default()
    }

    pub fn translate(dx: f64, dy: f64) -> Self {
        Self {
            translation: Vector2::new(dx, dy),
            ..Self::default()
        }
    }

    pub fn apply(&self, point: Point2) -> Point2 {
        let cos = self.rotation.cos() * self.scale;
        let sin = self.rotation.sin() * self.scale;
        Point2 {
            x: point.x * cos - point.y * sin + self.translation.x,
            y: point.x * sin + point.y * cos + self.translation.y,
        }
    }

    pub fn apply_inverse(&self, point: Point2) -> Point2 {
        let translated = Point2 {
            x: point.x - self.translation.x,
            y: point.y - self.translation.y,
        };
        let cos = (self.rotation.cos() * self.scale).max(f64::EPSILON);
        let sin = self.rotation.sin() * self.scale;
        let det = cos * cos + sin * sin;
        Point2 {
            x: (translated.x * cos + translated.y * sin) / det,
            y: (-translated.x * sin + translated.y * cos) / det,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_roundtrip() {
        let t = Transform2 {
            translation: Vector2::new(10.0, 5.0),
            rotation: 0.5,
            scale: 2.0,
        };
        let p = Point2::new(3.0, 4.0);
        let round = t.apply_inverse(t.apply(p));
        assert!((round.x - p.x).abs() < 1e-9);
        assert!((round.y - p.y).abs() < 1e-9);
    }
}
