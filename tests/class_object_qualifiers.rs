use laml::{export, geometry::place, parser::parse, route::route, styles::StyleSheet, svg::render};

#[test]
fn qualifier_boxes_attach_to_association_ends_in_all_output_formats() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass Kunde\nclass Auftrag\n@orders Kunde qualifier(id:UUID,region:String) -- qualifier(nr:Integer) Auftrag\n}\nlayout {\nKunde < Auftrag : align=center; gap=350\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(
        diagram.relations[0]
            .source_end
            .qualifier
            .as_ref()
            .unwrap()
            .len(),
        2
    );
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    assert_eq!(routes[0].qualifiers.len(), 2);
    let (first, last) = (&routes[0].qualifiers[0], &routes[0].qualifiers[1]);
    assert!((first.x - boxes[0].x - boxes[0].width).abs() < 1e-6);
    assert!((last.x + last.width - boxes[1].x).abs() < 1e-6);
    assert!((routes[0].points[0].x - first.x - first.width).abs() < 1e-6);
    assert!((routes[0].points.last().unwrap().x - last.x).abs() < 1e-6);
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("id: UUID"));
    assert!(svg.contains("nr: Integer"));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn qualifiers_fail_if_they_misrepresent_a_dependency_or_object_link() {
    let base = "settings {\ntype: class\n}\nsemantics {\nA\nB\nA qualifier(id:UUID) -- B\n}\n";
    let dependency = base.replace(" -- B", " ..> B");
    assert!(
        parse(&dependency)
            .unwrap_err()
            .message
            .contains("association-end")
    );
    let object = base.replace("type: class", "type: object");
    assert!(
        parse(&object)
            .unwrap_err()
            .message
            .contains("unsupported qualifier")
    );
    let invalid = base.replace("id:UUID", "id:");
    assert!(
        parse(&invalid)
            .unwrap_err()
            .message
            .contains("invalid qualifier")
    );
}
