use laml::{export, geometry::place, parser::parse, route::route, styles::StyleSheet, svg::render};

#[test]
fn nary_association_is_one_small_diamond_with_three_distinct_arms() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass Arzt\nclass Patient\nclass Medikament\n@verordnet association\nArzt -- verordnet\nPatient -- verordnet\nMedikament -- verordnet\n}\n";
    let diagram = parse(source).unwrap();
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    assert_eq!((boxes[3].width, boxes[3].height), (24.0, 24.0));
    assert!(boxes[3].lines.is_empty());
    let routed = route(&diagram, &boxes, &styles).unwrap();
    assert_eq!(routed.len(), 3);
    let svg = render(&boxes, &routed).unwrap();
    assert_eq!(svg.matches("<polygon ").count(), 1);
    assert!(svg.contains("data-node-id=\"verordnet\""));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
}

#[test]
fn association_class_attaches_its_class_box_to_one_plain_binary_association() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass Kunde\nclass Bestellung\nassociation class Auftrag {\nid: UUID\n}\n@orders Kunde -- Bestellung\nAuftrag .. relation[orders]\n}\nlayout {\nKunde < Bestellung : align=center; gap=240\nAuftrag above Kunde : gap=90\n}\n";
    let diagram = parse(source).unwrap();
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routed = route(&diagram, &boxes, &styles).unwrap();
    assert_eq!(routed.len(), 2);
    let svg = render(&boxes, &routed).unwrap();
    assert!(svg.contains("Auftrag"));
    assert!(svg.contains("id: UUID"));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
    let disconnected = source.replace("Auftrag .. relation[orders]", "");
    assert!(
        parse(&disconnected)
            .unwrap_err()
            .message
            .contains("exactly one")
    );
    let dependency = source.replace("Kunde -- Bestellung", "Kunde ..> Bestellung");
    assert!(
        parse(&dependency)
            .unwrap_err()
            .message
            .contains("plain binary")
    );
    let twice = source.replace(
        "Auftrag .. relation[orders]",
        "Auftrag .. relation[orders]\nAuftrag .. relation[orders]",
    );
    assert!(parse(&twice).unwrap_err().message.contains("exactly one"));
}

#[test]
fn nary_association_rejects_binary_or_semantically_marked_arms() {
    let base = "settings {\ntype: class\n}\nsemantics {\nclass A\nclass B\nclass C\n@j association\nA -- j\nB -- j\nC -- j\n}\n";
    assert!(
        parse(&base.replace("C -- j\n", ""))
            .unwrap_err()
            .message
            .contains("three distinct")
    );
    assert!(
        parse(&base.replace("C -- j", "A -- j"))
            .unwrap_err()
            .message
            .contains("three distinct")
    );
    assert!(
        parse(&base.replace("A -- j", "A *-- j"))
            .unwrap_err()
            .message
            .contains("plain, unlabeled")
    );
    assert!(
        parse(&base.replace("A -- j", "A -- j : owns"))
            .unwrap_err()
            .message
            .contains("plain, unlabeled")
    );
    let wrong = base.replace("@j association", "association j");
    assert!(
        parse(&wrong)
            .unwrap_err()
            .message
            .contains("captions are not implemented")
    );
}
