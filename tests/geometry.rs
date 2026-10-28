use laml::geometry::place;
use laml::parser::parse;
use laml::styles::StyleSheet;

#[test]
fn a_growing_label_keeps_the_default_center_anchor() {
    let short = parse("semantics {\nA(label=\"API\")\n}\n").unwrap();
    let long = parse("semantics {\nA(label=\"A much longer API name\")\n}\n").unwrap();
    let left = place(&short, &StyleSheet::default()).unwrap();
    let right = place(&long, &StyleSheet::default()).unwrap();
    assert!(right[0].width > left[0].width);
    assert!(((right[0].x + right[0].width / 2.0) - (left[0].x + left[0].width / 2.0)).abs() < 1e-6);
}

#[test]
fn combined_alignment_and_separation_are_hard() {
    let diagram = parse("semantics {\nA\nB\nC\n}\nlayout {\nA < B : align=center; gap=24\nC below B : align=left\n}\n").unwrap();
    let boxes = place(&diagram, &StyleSheet::default()).unwrap();
    assert!(boxes[1].x >= boxes[0].x + boxes[0].width + 24.0 - 1e-6);
    assert!((boxes[0].y + boxes[0].height / 2.0 - boxes[1].y - boxes[1].height / 2.0).abs() < 1e-6);
    assert!(boxes[2].y >= boxes[1].y + boxes[1].height + 24.0 - 1e-6);
    assert!((boxes[2].x - boxes[1].x).abs() < 1e-6);
}

#[test]
fn adding_a_relation_to_later_nodes_preserves_earlier_unrelated_centers() {
    let before = parse("semantics {\nA\nB\nC\nD\n}\n").unwrap();
    let after = parse("semantics {\nA\nB\nC\nD\nC -> D\n}\n").unwrap();
    let a = place(&before, &StyleSheet::default()).unwrap();
    let b = place(&after, &StyleSheet::default()).unwrap();
    for i in 0..2 {
        assert_eq!((a[i].x, a[i].y), (b[i].x, b[i].y));
    }
}

#[test]
fn late_cycle_closure_does_not_reposition_a_twenty_node_chain() {
    let mut source = "semantics {\n".to_owned();
    for i in 0..20 {
        source.push_str(&format!("N{i:02}\n"));
    }
    for i in 0..19 {
        source.push_str(&format!("N{i:02} -> N{:02}\n", i + 1));
    }
    let before = parse(&format!("{source}}}\n")).unwrap();
    let after = parse(&format!("{source}N19 -> N00\n}}\n")).unwrap();
    let styles = StyleSheet::default();
    let a = place(&before, &styles).unwrap();
    let b = place(&after, &styles).unwrap();
    for (left, right) in a.iter().zip(b.iter()) {
        assert_eq!(
            (left.x, left.y, left.width, left.height),
            (right.x, right.y, right.width, right.height)
        );
    }
}

#[test]
fn impossible_same_axis_alignment_and_separation_fail() {
    let diagram = parse("semantics {\nA\nB\nC\n}\nlayout {\nA < B : align=top\nB < C : align=top\nC below A : align=left\nA below C\n}\n").unwrap();
    assert!(place(&diagram, &StyleSheet::default()).is_err());
}
