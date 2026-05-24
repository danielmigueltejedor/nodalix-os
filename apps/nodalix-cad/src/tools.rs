#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tool {
    Select,
    Line,
    Polyline,
    Rectangle,
    Circle,
    Arc,
    Dimension,
    Text,
    Measure,
    Modify,
    Block,
    Hatch,
    Table,
    Parametric,
    Guideline,
    Pan,
    Orbit,
    SectionFromMesh,
    FitCurve,
    Extrude,
}

impl Tool {
    pub fn all() -> &'static [(Tool, &'static str, &'static str)] {
        &[
            (Tool::Select, "󰆾", "Select"),
            (Tool::Line, "╱", "Line"),
            (Tool::Polyline, "󰕕", "Polyline"),
            (Tool::Rectangle, "󰹞", "Rectangle"),
            (Tool::Circle, "󰝦", "Circle"),
            (Tool::Arc, "󰘦", "Arc"),
            (Tool::Dimension, "󰑭", "Dimension"),
            (Tool::Text, "󰉿", "Text"),
            (Tool::Measure, "󰋊", "Measure"),
            (Tool::Modify, "󰏫", "Modify"),
            (Tool::Block, "󰆧", "Blocks"),
            (Tool::Hatch, "▨", "Hatch"),
            (Tool::Table, "󰓫", "Table"),
            (Tool::Parametric, "󰘩", "Parametric"),
            (Tool::Guideline, "╍", "Guidelines"),
            (Tool::Pan, "󰆾", "Pan"),
            (Tool::Orbit, "󰆧", "Orbit placeholder"),
            (Tool::SectionFromMesh, "󰚟", "Section from Mesh placeholder"),
            (Tool::FitCurve, "󰤨", "Fit Curve placeholder"),
            (Tool::Extrude, "󰆦", "Extrude placeholder"),
        ]
    }

    pub fn label(self) -> &'static str {
        Self::all()
            .iter()
            .find(|(tool, _, _)| *tool == self)
            .map(|(_, _, label)| *label)
            .unwrap_or("Select")
    }
}
