use laml::{sequence, styles::StyleSheet, svg::TextMode};

fn diagram(events: &str, layout: &str) -> String {
    format!(
        "settings {{\n type: sequence\n}}\nsemantics {{\n A\n B\n{events}\n}}\nlayout {{\n{layout}\n}}\n"
    )
}
#[test]
fn both_activation_edges_align_and_keep_source_identity() {
    let source = diagram(
        " first: A+\n second: B+\n B-\n A-",
        " A.first+ ? B.second+ : align=center-y\n A.first- ? B.second- : align=center-y",
    );
    let doc = sequence::parse(&source).unwrap();
    let (_, regions) =
        sequence::render::render_for_editor(&doc, &StyleSheet::default(), TextMode::System)
            .unwrap();
    let bars: Vec<_> = regions.iter().filter(|r| r.kind == "execution").collect();
    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].y, bars[1].y);
    assert_eq!(bars[0].y + bars[0].height, bars[1].y + bars[1].height);
    assert!(bars.iter().all(|bar| bar.end_line.is_some()));
}
#[test]
fn bound_message_moves_with_the_activation_start() {
    let source = diagram(
        " first: A+\n call: A -> B+ : work\n B-\n A-",
        " A.first+ ? B.call+ : align=center-y",
    );
    let doc = sequence::parse(&source).unwrap();
    let (_, regions) =
        sequence::render::render_for_editor(&doc, &StyleSheet::default(), TextMode::System)
            .unwrap();
    let bars: Vec<_> = regions.iter().filter(|r| r.kind == "execution").collect();
    assert_eq!(bars[0].y, bars[1].y);
    let receive = regions
        .iter()
        .find(|r| r.kind == "sequence-receive")
        .unwrap();
    assert_eq!(receive.y, bars[0].y);
}
#[test]
fn dragged_bottom_can_shrink_or_grow_without_moving_its_reference() {
    let events = " first: A+\n second: B+\n A -> B : Foo\n B-\n A-";
    let edges = |layout: &str| {
        let doc = sequence::parse(&diagram(events, layout)).unwrap();
        let (_, regions) =
            sequence::render::render_for_editor(&doc, &StyleSheet::default(), TextMode::System)
                .unwrap();
        let edge = |owner: &str| {
            let bar = regions
                .iter()
                .find(|r| r.kind == "sequence-execution" && r.id == owner)
                .unwrap();
            bar.y + bar.height
        };
        (edge("A"), edge("B"))
    };
    let base = edges("");
    assert!(base.0 > base.1);
    let shortened = edges(" A.first- ? B.second- : align=center-y; reference=second");
    assert_eq!(shortened, (base.1, base.1));
    let extended = edges(" B.second- ? A.first- : align=center-y; reference=second");
    assert_eq!(extended, (base.0, base.0));
}
#[test]
fn ambiguous_and_mismatched_edges_fail() {
    assert!(
        sequence::parse(&diagram(
            " A+\n A-\n A+\n A-\n B+\n B-",
            " A- ? B- : align=center-y"
        ))
        .is_err()
    );
    assert!(sequence::parse(&diagram(" A+\n B+\n A-\n B-", " A+ ? B- : align=center-y")).is_err());
}
