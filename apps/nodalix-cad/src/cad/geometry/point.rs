use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

impl Point2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn translate(self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox2 {
    pub min: Point2,
    pub max: Point2,
}

impl BoundingBox2 {
    pub fn empty() -> Self {
        Self {
            min: Point2 {
                x: f64::INFINITY,
                y: f64::INFINITY,
            },
            max: Point2 {
                x: f64::NEG_INFINITY,
                y: f64::NEG_INFINITY,
            },
        }
    }

    pub fn from_points(points: &[Point2]) -> Option<Self> {
        let mut bbox = Self::empty();
        for point in points {
            bbox.include(*point);
        }
        bbox.is_valid().then_some(bbox)
    }

    pub fn include(&mut self, point: Point2) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
    }

    pub fn merge(self, other: Self) -> Self {
        Self {
            min: Point2 {
                x: self.min.x.min(other.min.x),
                y: self.min.y.min(other.min.y),
            },
            max: Point2 {
                x: self.max.x.max(other.max.x),
                y: self.max.y.max(other.max.y),
            },
        }
    }

    pub fn is_valid(&self) -> bool {
        self.min.x.is_finite()
            && self.min.y.is_finite()
            && self.max.x.is_finite()
            && self.max.y.is_finite()
    }

    pub fn width(&self) -> f64 {
        (self.max.x - self.min.x).abs()
    }

    pub fn height(&self) -> f64 {
        (self.max.y - self.min.y).abs()
    }

    pub fn center(&self) -> Point2 {
        Point2 {
            x: (self.min.x + self.max.x) * 0.5,
            y: (self.min.y + self.max.y) * 0.5,
        }
    }

    pub fn contains(&self, point: Point2) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    pub fn expand(&self, margin: f64) -> Self {
        Self {
            min: Point2 {
                x: self.min.x - margin,
                y: self.min.y - margin,
            },
            max: Point2 {
                x: self.max.x + margin,
                y: self.max.y + margin,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_distance() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(3.0, 4.0);
        assert!((a.distance_to(b) - 5.0).abs() < 1e-9);
    }

    #[test]
    fn bounding_box_from_points() {
        let bbox =
            BoundingBox2::from_points(&[Point2::new(1.0, 2.0), Point2::new(-1.0, 5.0)]).unwrap();
        assert_eq!(bbox.min, Point2::new(-1.0, 2.0));
        assert_eq!(bbox.max, Point2::new(1.0, 5.0));
    }

    #[test]
    fn bounding_box_intersects() {
        let a = BoundingBox2::from_points(&[Point2::new(0.0, 0.0), Point2::new(2.0, 2.0)]).unwrap();
        let b = BoundingBox2::from_points(&[Point2::new(1.0, 1.0), Point2::new(3.0, 3.0)]).unwrap();
        assert!(a.intersects(&b));
    }
}
