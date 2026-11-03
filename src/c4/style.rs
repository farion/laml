//! Classic, readable C4 defaults. User LSS is appended afterwards and wins on ties.
use crate::parser::{self, ParseError};
use crate::syntax::{Diagram, StyleEntry};

pub(super) fn apply(diagram: &mut Diagram) -> Result<(), ParseError> {
    let dark = diagram.settings.theme.dark();
    let fill = if dark { "#10151d" } else { "#ffffff" };
    let inner = if dark { "#17212b" } else { "#f8fbff" };
    let ink = if dark { "#edf4fc" } else { "#1b3345" };
    let blue = if dark { "#75baff" } else { "#176aa7" };
    let green = if dark { "#89d183" } else { "#298118" };
    let red = if dark { "#ff8585" } else { "#b82935" };
    let source = format!(
        r#"styling {{
        node.c4Person {{
            fill: "{fill}"
            color: "{green}"
            border-color: "{green}"
            border-width: 2
            min-width: 270
            padding: 20
        }}
        node.c4System {{
            fill: "{fill}"
            color: "{blue}"
            border-color: "{blue}"
            border-width: 2
            min-width: 230
            padding: 20
        }}
        node.c4Container {{
            fill: "{fill}"
            color: "{blue}"
            border-color: "{blue}"
            border-width: 2
            min-width: 215
            padding: 20
        }}
        node.c4Component {{
            fill: "{fill}"
            color: "{blue}"
            border-color: "{blue}"
            border-width: 2
            min-width: 210
            padding: 20
        }}
        node.c4Boundary {{
            fill: "{inner}"
            color: "{blue}"
            border-color: "{blue}"
            border-width: 2
            padding: 20
        }}
        node.c4External {{
            fill: "{fill}"
            color: "{red}"
            border-color: "{red}"
            border-width: 2
        }}
        relation {{
            color: "{ink}"
            width: 2
        }}
    }}"#
    );
    let mut defaults: Vec<StyleEntry> = parser::parse_stylesheet(&source)?;
    defaults.append(&mut diagram.styling);
    diagram.styling = defaults;
    Ok(())
}
