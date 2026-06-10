use super::point::Point2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Vector2 {
    pub x: f64,
    pub y: f64,
}

impl Vector2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn from_points(from: Point2, to: Point2) -> Self {
        Self {
            x: to.x - from.x,
            y: to.y - from.y,
        }
    }

    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn length_squared(self) -> f64 {
        self.x * self.x + self.y * self.y
    }

    pub fn normalize(self) -> Option<Self> {
        let len = self.length();
        if len <= f64::EPSILON {
            return None;
        }
        Some(Self {
            x: self.x / len,
            y: self.y / len,
        })
    }

    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }

    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    pub fn perpendicular(self) -> Self {
        Self {
            x: -self.y,
            y: self.x,
        }
    }
}

impl std::ops::Add for Vector2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub for Vector2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Add<Vector2> for Point2 {
    type Output = Point2;

    fn add(self, rhs: Vector2) -> Self::Output {
        Point2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl std::ops::Sub<Vector2> for Point2 {
    type Output = Point2;

    fn sub(self, rhs: Vector2) -> Self::Output {
        Point2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl std::ops::Mul<f64> for Vector2 {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_length_and_dot() {
        let v = Vector2::new(3.0, 4.0);
        assert!((v.length() - 5.0).abs() < 1e-9);
        assert!((v.dot(Vector2::new(1.0, 0.0)) - 3.0).abs() < 1e-9);
    }

    #[test]
    fn vector_normalize() {
        let n = Vector2::new(10.0, 0.0).normalize().unwrap();
        assert!((n.x - 1.0).abs() < 1e-9);
    }
}
