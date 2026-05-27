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
            (Tool::Select, "select", "Select"),
            (Tool::Line, "line", "Line"),
            (Tool::Polyline, "polyline", "Polyline"),
            (Tool::Rectangle, "rectangle", "Rectangle"),
            (Tool::Circle, "circle", "Circle"),
            (Tool::Arc, "arc", "Arc"),
            (Tool::Dimension, "dimension", "Dimension"),
            (Tool::Text, "text", "Text"),
            (Tool::Measure, "measure", "Measure"),
            (Tool::Modify, "move", "Modify"),
            (Tool::Block, "block", "Blocks"),
            (Tool::Hatch, "hatch", "Hatch"),
            (Tool::Table, "table", "Table"),
            (Tool::Parametric, "parametric", "Parametric"),
            (Tool::Guideline, "guideline", "Guidelines"),
            (Tool::Pan, "pan", "Pan"),
            (Tool::Orbit, "orbit", "Orbit"),
            (Tool::SectionFromMesh, "section", "Section"),
            (Tool::FitCurve, "fit-curve", "Fit Curve"),
            (Tool::Extrude, "extrude", "Extrude"),
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
