//! SVG marker lookup; semantics choose the marker before rendering.
use crate::syntax::EndSymbol;

pub(super) fn marker(symbol: EndSymbol, start: bool) -> &'static str {
    match (symbol, start) {
        (EndSymbol::None, _) => "",
        (EndSymbol::Arrow, _) => "url(#arrow)",
        (EndSymbol::Triangle, _) => "url(#triangle)",
        (EndSymbol::OpenDiamond, true) => "url(#open-diamond)",
        (EndSymbol::OpenDiamond, false) => "url(#open-diamond-end)",
        (EndSymbol::FilledDiamond, true) => "url(#filled-diamond)",
        (EndSymbol::FilledDiamond, false) => "url(#filled-diamond-end)",
    }
}
