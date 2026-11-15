//! Connections must not fail merely because the preferred local lanes are crowded.
use laml::{geometry, parser::parse, route, styles::StyleSheet};

#[test]
fn two_nodes_three_arrows_with_automatic_and_fixed_sides() {
    for rules in [
        "",
        "relation[a].source : side=right\nrelation[a].target : side=left\nrelation[b].source : side=right\nrelation[b].target : side=left\nrelation[c].source : side=right\nrelation[c].target : side=left\n",
    ] {
        let source = format!(
            "semantics {{\nA\nB\n@a A -> B\n@b A -> B\n@c A -> B\n}}\nlayout {{\n{rules}}}\n"
        );
        let d = parse(&source).unwrap();
        let s = StyleSheet::default();
        let b = geometry::place(&d, &s).unwrap();
        let r = route::route(&d, &b, &s);
        assert!(r.is_ok(), "{source}\n{r:?}");
        assert_eq!(r.unwrap().len(), 3);
    }
}

#[test]
fn blocked_exit_expands_spacing_but_preserves_authored_order_and_alignment() {
    let source = "semantics {\nA\nC\nB\n@r A -> B\n}\nlayout {\nA < C : align=center; gap=0\nC < B : align=center; gap=0\nrelation[r].source : side=right\nrelation[r].target : side=left\n}\n";
    let d = parse(source).unwrap();
    let s = StyleSheet::default();
    let before = geometry::place(&d, &s).unwrap();
    assert!(
        route::route(&d, &before, &s).is_err(),
        "fixed geometry has a blocked mandatory exit"
    );
    let (b, r) = route::place_and_route(&d, &s).unwrap();
    assert_eq!(r.len(), 1);
    let (a, c, z) = (&b[0], &b[1], &b[2]);
    assert!(c.x >= a.x + a.width + 31.99);
    assert!(z.x >= c.x + c.width + 31.99);
    assert!((a.y + a.height / 2.0 - c.y - c.height / 2.0).abs() < 1e-6);
    assert!((c.y + c.height / 2.0 - z.y - z.height / 2.0).abs() < 1e-6);
    assert_eq!(r[0].points[0].x, a.x + a.width);
    assert_eq!(r[0].points.last().unwrap().x, z.x);
    let (again, r2) = route::place_and_route(&d, &s).unwrap();
    assert_eq!(
        b.iter().map(|b| (b.x, b.y)).collect::<Vec<_>>(),
        again.iter().map(|b| (b.x, b.y)).collect::<Vec<_>>()
    );
    assert_eq!(r[0].points, r2[0].points);
    assert_eq!(d.layout[0].gap, Some(0.0));
}

#[test]
fn editor_sized_parallel_arrow_matrix() {
    for count in 2..=3 {
        for op in ["->", "<->", "..>"] {
            for direction in ["right", "left", "bottom", "top"] {
                let mut source = String::from("semantics {\nA\nB\n");
                for i in 0..count {
                    source.push_str(&format!("@r{i} A {op} B\n"));
                }
                source.push_str("}\nlayout {\n");
                for i in 0..count {
                    source.push_str(&format!("relation[r{i}].source : side={direction}\n"));
                }
                source.push_str("}\n");
                let d = parse(&source).unwrap();
                let result = route::place_and_route(&d, &StyleSheet::default());
                assert!(result.is_ok(), "{source}\n{result:?}");
            }
        }
    }
}

#[test]
fn spacing_rescue_also_applies_inside_containers() {
    let d=parse("semantics {\npackage Frame {\n@A A\n@C C\n@B B\n}\n@r A -> B\n}\nlayout {\nA < C : align=center; gap=0\nC < B : align=center; gap=0\nrelation[r].source : side=right\nrelation[r].target : side=left\n}\n").unwrap();
    let s = StyleSheet::default();
    let before = geometry::place(&d, &s).unwrap();
    assert!(route::route(&d, &before, &s).is_err());
    let (boxes, routes) = route::place_and_route(&d, &s).unwrap();
    let a = boxes.iter().find(|b| b.id == "A").unwrap();
    let c = boxes.iter().find(|b| b.id == "C").unwrap();
    let frame = boxes.iter().find(|b| b.id == "Frame").unwrap();
    assert!(c.x >= a.x + a.width + 31.99);
    assert!(frame.width > before.iter().find(|b| b.id == "Frame").unwrap().width);
    assert_eq!(routes.len(), 1);
    laml::svg::render(&boxes, &routes).unwrap();
}

#[test]
fn spacing_rescue_cannot_enlarge_an_explicitly_fixed_container() {
    let source = "semantics {\npackage Frame {\n@A A\n@C C\n@B B\n}\n@r A -> B\n}\nlayout {\nA < C : align=center; gap=0\nC < B : align=center; gap=0\nrelation[r].source : side=right\nrelation[r].target : side=left\n}\nstyling {\nnode[Frame] {\nwidth: 400\nheight: 200\n}\n}\n";
    let d = parse(source).unwrap();
    let s = laml::styles::load(&d, std::path::Path::new("fixed-routing.laml")).unwrap();
    let before = geometry::place(&d, &s).unwrap();
    assert_eq!(
        before.iter().find(|b| b.id == "Frame").unwrap().width,
        400.0
    );
    let result = route::place_and_route(&d, &s);
    assert!(
        result.is_err(),
        "spacing must not enlarge a fixed frame: {result:?}"
    );
    let after = geometry::place(&d, &s).unwrap();
    assert_eq!(
        before
            .iter()
            .map(|b| (b.x, b.y, b.width, b.height))
            .collect::<Vec<_>>(),
        after
            .iter()
            .map(|b| (b.x, b.y, b.width, b.height))
            .collect::<Vec<_>>()
    );
}
