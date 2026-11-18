use laml::{authoring::Region, sequence, styles::StyleSheet, svg::TextMode};
fn render(source: &str) -> (String, Vec<Region>) {
    let doc = sequence::parse(source).unwrap();
    sequence::render::render_for_editor(&doc, &StyleSheet::default(), TextMode::System).unwrap()
}
fn edge<'a>(regions: &'a [Region], name: &str) -> &'a Region {
    regions
        .iter()
        .find(|r| r.kind == "message" && r.id == name && !r.path.is_empty())
        .unwrap()
}
fn crossing(a: (f64, f64), b: (f64, f64), c: (f64, f64), d: (f64, f64)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let (ox, oy) = (d.0 - c.0, d.1 - c.1);
    let det = dx * oy - dy * ox;
    if det.abs() < 1e-6 {
        return false;
    }
    let t = ((c.0 - a.0) * oy - (c.1 - a.1) * ox) / det;
    let u = ((c.0 - a.0) * dy - (c.1 - a.1) * dx) / det;
    (0.001..0.999).contains(&t) && (0.001..0.999).contains(&u)
}
#[test]
fn nested_activation_self_messages_use_inner_and_outer_corridors() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n Service\n outer: Service+ -> Service\n Service -> Service\n inner: Service+\n nested: Service -> Service\n Service-\n Service-\n}\nlayout {\n outer.receive ? Service.outer- : align=center-y; reference=second\n nested.receive ? Service.inner- : align=center-y; reference=second\n}\n";
    let (_, regions) = render(source);
    let outer = edge(&regions, "outer");
    let inner = edge(&regions, "nested");
    assert!(outer.path[0].1 < inner.path[0].1 && outer.path[3].1 > inner.path[3].1);
    assert!(outer.path[1].0 >= inner.path[1].0 + 16.0);
    for a in outer.path.windows(2) {
        for b in inner.path.windows(2) {
            assert!(!crossing(a[0], a[1], b[0], b[1]));
        }
    }
    assert!(outer.pick_paths.is_empty() && inner.pick_paths.is_empty());
}
#[test]
fn unavoidable_crossing_uses_shared_bridge_and_real_picking_paths() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n C\n loop: B -> B\n crossing: A -> C\n tail: B+\n B-\n}\nlayout {\n loop.receive ? B.tail- : align=center-y; reference=second\n}\n";
    let (svg, regions) = render(source);
    let a = edge(&regions, "loop");
    let b = edge(&regions, "crossing");
    assert!(
        svg.contains(" Q"),
        "A clear crossing should use the shared bridge arc"
    );
    assert!(a.pick_paths.len() >= 2 && !b.pick_paths.is_empty());
    for ap in &a.pick_paths {
        for bp in &b.pick_paths {
            for aa in ap.windows(2) {
                for bb in bp.windows(2) {
                    assert!(!crossing(aa[0], aa[1], bb[0], bb[1]));
                }
            }
        }
    }
}
#[test]
fn interleaving_self_calls_have_distinct_corridors_and_compact_underpasses() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n first: A -> A\n second: A -> A\n early: A+\n A-\n late: A+\n A-\n}\nlayout {\n first.receive ? A.early- : align=center-y; reference=second\n second.receive ? A.late- : align=center-y; reference=second\n}\n";
    let (_, regions) = render(source);
    let first = edge(&regions, "first");
    let second = edge(&regions, "second");
    assert!((first.path[1].0 - second.path[1].0).abs() >= 16.0);
    assert!(!first.pick_paths.is_empty() || !second.pick_paths.is_empty());
}
#[test]
fn self_corridors_and_captions_fit_before_the_next_participant() {
    let source = "settings {\n type: sequence\n}\nsemantics {\n A\n B\n outer: A+ -> A: outer caption\n inner: A -> A: a substantially wider inner caption\n A-\n}\nlayout {\n outer.receive ? A.outer- : align=center-y; reference=second\n}\n";
    let (_, regions) = render(source);
    let b = regions
        .iter()
        .find(|r| r.id == "B" && r.kind == "participant")
        .unwrap();
    for r in regions.iter().filter(|r| r.kind == "message") {
        assert!(
            r.x + r.width < b.x,
            "Routes and captions must not enter the next participant's column"
        );
    }
}

#[test]
fn deep_self_corridors_fit_fragment_and_inline_decomposition_bounds() {
    let mut body = " A\n loop \"nested\" {\n".to_string();
    let mut rules = String::new();
    for i in 0..4 {
        body.push_str(&format!(" call{i}: A+ -> A: nested self call number {i}\n"));
        rules.push_str(&format!(
            " call{i}.receive ? A.call{i}- : align=center-y; reference=second\n"
        ));
    }
    body.push_str(" A-\n A-\n A-\n A-\n }\n");
    let source =
        format!("settings {{\n type: sequence\n}}\nsemantics {{\n{body}}}\nlayout {{\n{rules}}}\n");
    let (_, regions) = render(&source);
    let frame = regions.iter().find(|r| r.kind == "frame").unwrap();
    for r in regions.iter().filter(|r| r.kind == "message") {
        assert!(r.x + r.width <= frame.x + frame.width);
    }
    let qualified = rules
        .replace(" call", " Root.call")
        .replace("? A.", "? Root.A.");
    let nested = format!(
        "settings {{\n type: sequence\n}}\nsemantics {{\n Root\n Other\n decompose Root \"Deep\" {{\n{body} }}\n}}\nlayout {{\n{qualified}}}\n"
    );
    let (_, regions) = render(&nested);
    let root = regions
        .iter()
        .find(|r| r.kind == "participant" && r.id == "Root")
        .unwrap();
    for r in regions
        .iter()
        .filter(|r| r.kind == "message" && r.id.starts_with("Root."))
    {
        assert!(
            r.x >= root.x && r.x + r.width <= root.x + root.width,
            "Nested routes must stay inside the parent's allocated column"
        );
    }
}
