use laml::geometry::place;
use laml::parser::parse;
use laml::route::route;
use laml::styles::StyleSheet;
use laml::svg::{TextMode, render, render_with_theme};
use laml::syntax::LabelPlacement;
use laml::theme::Theme;

#[test]
fn global_inline_and_named_along_override_render_distinctly() {
    let source = include_str!("../examples/label-modes.laml");
    let diagram = parse(source).unwrap();
    assert_eq!(diagram.settings.relation_label, LabelPlacement::Inline);
    assert_eq!(diagram.relation_layout[0].name, "along");
    let sheet = StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    assert_eq!(
        routes[0].label.as_ref().unwrap().placement,
        LabelPlacement::Inline
    );
    assert_eq!(routes[0].gaps.len(), 1);
    assert_eq!(
        routes[1].label.as_ref().unwrap().placement,
        LabelPlacement::Along
    );
    assert!(routes[1].label.as_ref().unwrap().angle > 10.0);
    let svg = render(&boxes, &routes).unwrap();
    assert!(svg.contains("rotate("));
    assert!(svg.contains(">Batch</text>"));
    assert!(svg.contains(">Stream</text>"));
}

#[test]
fn modern_along_pill_wraps_text_not_rotated_collision_envelope() {
    let diagram = parse(include_str!("../examples/label-modes.laml")).unwrap();
    let sheet = StyleSheet::default();
    let boxes = place(&diagram, &sheet).unwrap();
    let routes = route(&diagram, &boxes, &sheet).unwrap();
    let along = routes
        .iter()
        .find(|route| {
            route
                .label
                .as_ref()
                .is_some_and(|label| label.angle.abs() > 1e-9)
        })
        .unwrap();
    let label = along.label.as_ref().unwrap();
    let text_width = laml::text::measure(&label.text, along.font_size).unwrap();
    let line_height = along.font_size * 1.25;
    let cx = label.x + label.width / 2.0;
    let cy = label.y + label.height / 2.0;
    let expected = format!(
        "<rect data-edge-label-pill=\"true\" x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\"",
        cx - text_width / 2.0 - 4.0,
        cy - line_height / 2.0 - 2.0,
        text_width + 8.0,
        line_height + 4.0
    );
    for theme in [Theme::Modern, Theme::ModernDark] {
        let svg = render_with_theme(&boxes, &routes, TextMode::Embedded, theme).unwrap();
        let pill = svg.lines().find(|line| line.contains(&expected)).unwrap();
        assert!(pill.contains("transform=\"rotate("), "{pill}");
        assert!(svg.contains(&format!(">{}</text>", label.text)));
    }
}

#[test]
fn changing_label_mode_does_not_reposition_nodes() {
    let source = include_str!("../examples/label-modes.laml");
    let inline = parse(source).unwrap();
    let near = parse(&source.replace("relation-label: inline", "relation-label: near")).unwrap();
    let sheet = StyleSheet::default();
    let a = place(&inline, &sheet).unwrap();
    let b = place(&near, &sheet).unwrap();
    for (left, right) in a.iter().zip(b.iter()) {
        assert_eq!(
            (left.x, left.y, left.width, left.height),
            (right.x, right.y, right.width, right.height)
        );
    }
}

#[test]
fn invalid_relation_layout_reports_its_source_line() {
    let base = "semantics {\nA\nB\nmqtt: A -> B : MQTT\n}\nlayout {\n{}\n}\n";
    for (rule, message) in [
        ("relation[missing] : label=along", "unknown named relation"),
        (
            "relation[mqtt] : label=sideways",
            "invalid relation label placement",
        ),
        ("relation[mqtt] : label=near; label=inline", "exactly one"),
    ] {
        let error = parse(&base.replace("{}", rule)).unwrap_err();
        assert_eq!(error.line, 7);
        assert!(error.message.contains(message), "{error:?}");
    }
    let no_label =
        "semantics {\nA\nB\nmqtt: A -> B\n}\nlayout {\nrelation[mqtt] : label=along\n}\n";
    assert!(parse(no_label).unwrap_err().message.contains("no label"));
    let duplicated = "semantics {\nA\nB\nmqtt: A -> B : MQTT\n}\nlayout {\nrelation[mqtt] : label=inline\nrelation[mqtt] : label=near\n}\n";
    let error = parse(duplicated).unwrap_err();
    assert_eq!(error.line, 8);
    assert!(error.message.contains("duplicate relation layout"));
    let global = "settings {\nrelation-label: sideways\n}\nsemantics {\nA\n}\n";
    assert!(
        parse(global)
            .unwrap_err()
            .message
            .contains("invalid relation-label")
    );
}
