//! SVG marker lookup; semantics choose the marker before rendering.
use crate::syntax::EndSymbol;

pub(super) fn marker(symbol: EndSymbol, start: bool) -> &'static str {
    match (symbol, start) {
        (EndSymbol::None, _) => "",
        (EndSymbol::Arrow, _) => "url(#arrow)",
        (EndSymbol::Triangle, _) => "url(#triangle)",
        (EndSymbol::FilledTriangle, _) => "url(#filled-triangle)",
        (EndSymbol::OpenDiamond, true) => "url(#open-diamond)",
        (EndSymbol::OpenDiamond, false) => "url(#open-diamond-end)",
        (EndSymbol::FilledDiamond, true) => "url(#filled-diamond)",
        (EndSymbol::FilledDiamond, false) => "url(#filled-diamond-end)",
    }
}

pub(super) fn definitions(svg: &mut String) {
    svg.push_str("<marker id=\"arrow\" markerWidth=\"12\" markerHeight=\"8\" refX=\"10\" refY=\"4\" orient=\"auto-start-reverse\" markerUnits=\"userSpaceOnUse\"><path d=\"M0,0 L10,4 L0,8\" fill=\"none\" stroke=\"context-stroke\" stroke-width=\"1.5\"/></marker>");
    svg.push_str("<marker id=\"triangle\" markerWidth=\"13\" markerHeight=\"11\" refX=\"12\" refY=\"5.5\" orient=\"auto-start-reverse\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,1 L12,5.5 L1,10 Z\" fill=\"white\" stroke=\"context-stroke\" stroke-width=\"1.5\"/></marker>");
    svg.push_str("<marker id=\"filled-triangle\" markerWidth=\"13\" markerHeight=\"11\" refX=\"12\" refY=\"5.5\" orient=\"auto-start-reverse\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,1 L12,5.5 L1,10 Z\" fill=\"context-stroke\" stroke=\"context-stroke\" stroke-width=\"1.5\"/></marker>");
    svg.push_str("<marker id=\"open-diamond\" markerWidth=\"17\" markerHeight=\"11\" refX=\"1\" refY=\"5.5\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,5.5 L8,1 L16,5.5 L8,10 Z\" fill=\"white\" stroke=\"context-stroke\" stroke-width=\"1.5\"/></marker>");
    svg.push_str("<marker id=\"filled-diamond\" markerWidth=\"17\" markerHeight=\"11\" refX=\"1\" refY=\"5.5\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,5.5 L8,1 L16,5.5 L8,10 Z\" fill=\"context-stroke\"/></marker>");
    svg.push_str("<marker id=\"open-diamond-end\" markerWidth=\"17\" markerHeight=\"11\" refX=\"16\" refY=\"5.5\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,5.5 L8,1 L16,5.5 L8,10 Z\" fill=\"white\" stroke=\"context-stroke\" stroke-width=\"1.5\"/></marker>");
    svg.push_str("<marker id=\"filled-diamond-end\" markerWidth=\"17\" markerHeight=\"11\" refX=\"16\" refY=\"5.5\" orient=\"auto\" markerUnits=\"userSpaceOnUse\"><path d=\"M1,5.5 L8,1 L16,5.5 L8,10 Z\" fill=\"context-stroke\"/></marker></defs>\n");
}
