use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Unit {
    Millimeters,
    Centimeters,
    Meters,
    Inches,
}

impl Unit {
    pub fn label(self) -> &'static str {
        match self {
            Unit::Millimeters => "mm",
            Unit::Centimeters => "cm",
            Unit::Meters => "m",
            Unit::Inches => "inch",
        }
    }
}
