use laml::geometry::place;
use laml::parser::parse;
use laml::styles::StyleSheet;

fn boxes(source: &str) -> Vec<laml::geometry::BoxNode> {
    place(&parse(source).unwrap(), &StyleSheet::default()).unwrap()
}

#[test]
fn every_symbol_and_word_placement_has_identical_geometry() {
    for (symbol, word, axis, reverse) in [
        ("<", "left of", 'x', false),
        (">", "right of", 'x', true),
        ("^", "above", 'y', false),
        ("v", "below", 'y', true),
    ] {
        let diagram =
            |rule: &str| format!("semantics {{\nA\nB\n}}\nlayout {{\nA {rule} B : gap=45\n}}\n");
        let a = boxes(&diagram(symbol));
        let b = boxes(&diagram(word));
        assert_eq!(
            (a[0].x, a[0].y, a[1].x, a[1].y),
            (b[0].x, b[0].y, b[1].x, b[1].y),
            "{word}"
        );
        let (first, second) = if reverse {
            (&a[1], &a[0])
        } else {
            (&a[0], &a[1])
        };
        if axis == 'x' {
            assert!(second.x >= first.x + first.width + 45.0 - 1e-6, "{word}");
        } else {
            assert!(second.y >= first.y + first.height + 45.0 - 1e-6, "{word}");
        }
    }
}

#[test]
fn all_direction_preferences_are_subordinate_to_mandatory_layout() {
    for direction in [
        "left-to-right",
        "right-to-left",
        "top-to-bottom",
        "bottom-to-top",
    ] {
        let source = format!(
            "settings {{\ndirection: {direction}\n}}\nsemantics {{\nA\nB\nA -> B\n}}\nlayout {{\nB < A : align=center; gap=80\n}}\n"
        );
        let result = boxes(&source);
        assert!(result[0].x >= result[1].x + result[1].width + 80.0 - 1e-6);
        assert!(
            (result[0].y + result[0].height / 2.0 - result[1].y - result[1].height / 2.0).abs()
                < 1e-6
        );
    }
}

#[test]
fn impossible_aligned_placement_never_falls_back_to_soft_hint() {
    let source =
        "semantics {\nA\nB\n}\nlayout {\nA < B : align=top; gap=100\nB < A : align=top\n}\n";
    let diagram = parse(source).unwrap();
    assert!(place(&diagram, &StyleSheet::default()).is_err());
}

#[test]
fn adding_late_unrelated_node_keeps_earlier_node_centers() {
    let before = boxes("semantics {\nA\nB\nA -> B\n}\n");
    let after = boxes("semantics {\nA\nB\nC\nA -> B\n}\n");
    for i in 0..2 {
        assert_eq!((before[i].x, before[i].y), (after[i].x, after[i].y));
    }
}
