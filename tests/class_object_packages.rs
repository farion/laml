use laml::{
    export,
    geometry::place,
    parser::parse,
    route::route,
    styles::{StyleSheet, load},
    svg::render,
};

#[test]
fn nested_packages_scope_names_and_enclose_children_without_blocking_own_edges() {
    let source = "settings {\ntype: class\n}\nsemantics {\npackage Vertrieb {\nclass Kunde\npackage Lager {\nclass Produkt\n}\nKunde --> Lager.Produkt\n}\nclass Bericht\nVertrieb.Kunde --> Bericht\n}\n";
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.nodes[1].id, "Vertrieb.Kunde");
    assert_eq!(diagram.nodes[2].id, "Vertrieb.Lager");
    assert_eq!(diagram.nodes[3].id, "Vertrieb.Lager.Produkt");
    assert_eq!(diagram.relations[0].from, "Vertrieb.Kunde");
    assert_eq!(diagram.relations[0].to, "Vertrieb.Lager.Produkt");
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    for (parent, child) in [(0, 1), (0, 2), (2, 3)] {
        assert!(boxes[child].x > boxes[parent].x);
        assert!(boxes[child].y > boxes[parent].y);
        assert!(boxes[child].x + boxes[child].width < boxes[parent].x + boxes[parent].width);
        assert!(boxes[child].y + boxes[child].height < boxes[parent].y + boxes[parent].height);
    }
    let routes = route(&diagram, &boxes, &styles).unwrap();
    assert_eq!(routes.len(), 2);
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("Vertrieb"));
    assert!(svg.find("data-node-id=\"Vertrieb\"").unwrap() < svg.find("<polyline ").unwrap());
    assert!(svg.contains("Produkt"));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn relations_inside_packages_keep_labels_visible_and_collision_checked() {
    let source = "settings {\ntype: class\n}\nsemantics {\npackage Verkauf {\nclass Kunde\nclass Auftrag\n@orders Kunde \"1\" -- \"0..*\" Auftrag : bestellt\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    assert!(routes[0].label.is_some());
    assert_eq!(routes[0].end_labels.len(), 2);
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("bestellt"));
    assert!(svg.contains("0..*"));
}

#[test]
fn contextual_styles_and_hidden_descendants_respect_package_scope() {
    let source = "semantics {\npackage P(style-classes=domain) {\nA\nB\n}\n}\nstyling {\nnode[P] node[A] {\nfill: \"#aa0000\"\n}\nnode.domain node[B] {\nfill: \"#00aa00\"\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    assert_eq!(boxes[1].fill, "#aa0000");
    assert_eq!(boxes[2].fill, "#00aa00");
    let hidden = source.replace("fill: \"#aa0000\"", "display: none");
    let diagram = parse(&hidden).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    let boxes = place(&diagram, &styles).unwrap();
    assert_eq!(boxes.len(), 2);
    assert!(!boxes.iter().any(|node| node.id == "P.A"));
}

#[test]
fn fixed_packages_reject_clipping_and_cross_scope_rules_project_safely() {
    let source =
        "semantics {\npackage P {\nA\n}\nB\n}\nstyling {\nnode[P] {\nwidth: 80\nheight: 60\n}\n}\n";
    let diagram = parse(source).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &styles)
            .unwrap_err()
            .message
            .contains("fixed container")
    );
    let cross = "semantics {\npackage P {\nA\n}\nB\n}\nlayout {\nP.A < B : gap=70\n}\n";
    let diagram = parse(cross).unwrap();
    let boxes = place(&diagram, &StyleSheet::default()).unwrap();
    assert!(boxes[1].x + boxes[1].width + 70.0 <= boxes[2].x + 1e-6);
    let alignment = cross.replace("gap=70", "align=center; gap=70");
    let diagram = parse(&alignment).unwrap();
    let boxes = place(&diagram, &StyleSheet::default()).unwrap();
    let first = boxes[1].y + boxes[1].height / 2.0;
    let second = boxes[2].y + boxes[2].height / 2.0;
    assert!((first - second).abs() < 1e-6);
    let nested = "semantics {\npackage P {\npackage Q {\nA\n}\n}\nB\n}\nlayout {\nP.Q.A above B : align=left; gap=40\n}\n";
    let diagram = parse(nested).unwrap();
    let boxes = place(&diagram, &StyleSheet::default()).unwrap();
    assert!((boxes[2].x - boxes[3].x).abs() < 1e-6);
    assert!(boxes[2].y + boxes[2].height + 40.0 <= boxes[3].y + 1e-6);
    let hidden = "semantics {\npackage P {\nA\n}\nB\nP.A --> B\n}\nstyling {\nnode[P] {\ndisplay: none\n}\n}\n";
    let diagram = parse(hidden).unwrap();
    let styles = load(&diagram, std::path::Path::new("examples/architecture.laml")).unwrap();
    assert!(
        place(&diagram, &styles)
            .unwrap_err()
            .message
            .contains("hidden node")
    );
}
