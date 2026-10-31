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
    let conflicting_name = source.replace("Kunde -- Bestellung", "Kunde -- Bestellung : bestellt");
    assert!(
        parse(&conflicting_name)
            .unwrap_err()
            .message
            .contains("same visible name")
    );
    let dependency = source.replace("Kunde -- Bestellung", "Kunde ..> Bestellung");
    assert!(
        parse(&dependency)
            .unwrap_err()
            .message
            .contains("plain binary")
    );
    let second_class = source.replace(
        "Auftrag .. relation[orders]",
        "Auftrag .. relation[orders]\n@two association class Auftrag\ntwo .. relation[orders]",
    );
    assert!(
        parse(&second_class)
            .unwrap_err()
            .message
            .contains("multiple association classes")
    );
    let twice = source.replace(
        "Auftrag .. relation[orders]",
        "Auftrag .. relation[orders]\nAuftrag .. relation[orders]",
    );
    assert!(parse(&twice).unwrap_err().message.contains("exactly one"));
}

#[test]
fn association_class_can_attach_to_a_named_nary_diamond() {
    let source = "settings {\ntype: class\n}\nsemantics {\nclass Arzt\nclass Patient\nclass Medikament\nassociation Verordnet\n@facts association class Verordnet {\nzeit: Date\n}\nArzt -- Verordnet\nPatient -- Verordnet\nMedikament -- Verordnet\nfacts .. Verordnet\n}\nlayout {\nfacts above Arzt : gap=65\n}\n";
    let diagram = parse(source).unwrap();
    let styles = StyleSheet::default();
    let boxes = place(&diagram, &styles).unwrap();
    let routes = route(&diagram, &boxes, &styles).unwrap();
    let svg = render(&boxes, &routes).unwrap();
    assert_eq!(routes.len(), 4);
    assert!(svg.contains("zeit: Date"));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(export::png(&svg).is_ok());
    assert!(export::pdf(&svg).is_ok());
    let wrong = source.replace("association class Verordnet", "association class Andere");
    assert!(
        parse(&wrong)
            .unwrap_err()
            .message
            .contains("same visible name")
    );
    let two_classes = source.replace(
        "facts .. Verordnet",
        "facts .. Verordnet\n@other association class Verordnet\nother .. Verordnet",
    );
    assert!(
        parse(&two_classes)
            .unwrap_err()
            .message
            .contains("multiple association classes")
    );
    let extra = source.replace(
        "facts .. Verordnet",
        "facts .. Verordnet\nfacts .. Verordnet",
    );
    assert!(parse(&extra).unwrap_err().message.contains("exactly one"));
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
    let named = base.replace("@j association", "association j");
    let diagram = parse(&named).unwrap();
    let boxes = place(&diagram, &StyleSheet::default()).unwrap();
    assert!(boxes[3].width > 24.0);
    let routes = route(&diagram, &boxes, &StyleSheet::default()).unwrap();
    assert!(render(&boxes, &routes).unwrap().contains(">j</text>"));
}
