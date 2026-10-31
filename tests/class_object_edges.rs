use laml::{export, geometry::place, parser::parse, route::route, styles::StyleSheet, svg::render};

#[test]
fn note_attaches_to_named_edge_without_bridge_or_gap_at_the_t_junction() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass A\nclass B\nnote Hinweis \"Gilt für beide\"\n@orders A -- B\n@annotation Hinweis .. relation[orders]\n}\nlayout {\nA < B : align=center; gap=240\nHinweis above A : gap=90\n}\n";
    let diagram = parse(source).unwrap();
    let sheet = StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(routes.len(), 2);
    assert!(routes[0].gaps.is_empty());
    assert!(routes[1].bridges.is_empty());
    let tip = routes[1].points.last().unwrap();
    let (a, b) = (routes[0].points[0], *routes[0].points.last().unwrap());
    assert!((tip.y - a.y).abs() < 1e-6 && tip.x > a.x && tip.x < b.x);
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("Gilt für beide"));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn edge_attachment_rejects_unresolved_or_misleading_notation() {
    let valid = "settings {\ntype: class\n}\nsemantics {\nA\nB\nnote N \"text\"\n@r A -- B\nN .. relation[r]\n}\n";
    assert!(parse(valid).is_ok());
    let unknown = valid.replace("relation[r]", "relation[absent]");
    assert!(
        parse(&unknown)
            .unwrap_err()
            .message
            .contains("unknown target relation")
    );
    let wrong_kind = valid.replace("N .. relation[r]", "A .. relation[r]");
    assert!(
        parse(&wrong_kind)
            .unwrap_err()
            .message
            .contains("must be a note")
    );
    let wrong_line = valid.replace("N .. relation[r]", "N -- relation[r]");
    assert!(
        parse(&wrong_line)
            .unwrap_err()
            .message
            .contains("edge attachments require")
    );
}
