//! Independent visual alignment, without authored ordering or separation.
#[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreeAlignment {
    Left,
    Right,
    Top,
    Bottom,
    CenterX,
    CenterY,
    Center,
}
impl FreeAlignment {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "left" => Self::Left,
            "right" => Self::Right,
            "top" => Self::Top,
            "bottom" => Self::Bottom,
            "center-x" => Self::CenterX,
            "center-y" => Self::CenterY,
            "center" => Self::Center,
            _ => return None,
        })
    }
    pub fn keyword(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::CenterX => "center-x",
            Self::CenterY => "center-y",
            Self::Center => "center",
        }
    }
    /// Fraction of the box extent on the X/Y axis; None means unconstrained.
    pub fn anchors(self) -> (Option<f64>, Option<f64>) {
        match self {
            Self::Left => (Some(0.0), None),
            Self::Right => (Some(1.0), None),
            Self::Top => (None, Some(0.0)),
            Self::Bottom => (None, Some(1.0)),
            Self::CenterX => (Some(0.5), None),
            Self::CenterY => (None, Some(0.5)),
            Self::Center => (Some(0.5), Some(0.5)),
        }
    }
}
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct AlignmentRule {
    pub first: String,
    pub second: String,
    pub align: FreeAlignment,
    pub line: usize,
}
