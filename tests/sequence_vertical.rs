use laml::{sequence, svg::TextMode};

fn coordinate(svg: &str, name: &str, endpoint: &str) -> f64 {
    let line = svg
        .lines()
        .find(|line| line.contains(&format!("data-message=\"{name}\"")))
        .unwrap();
    let path = line
        .split(" d=\"")
        .nth(1)
        .unwrap()
        .split('"')
        .next()
        .unwrap();
    let pair = if endpoint == "send" {
        path.split(' ').next().unwrap().trim_start_matches('M')
    } else {
        path.split('L').nth(1).unwrap()
    };
    pair.split(',').nth(1).unwrap().parse().unwrap()
}

#[test]
fn vertical_coregion_gap_is_mandatory_and_stretches_visible_canvas() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nAPI\nfirst: A -> API : request\nsecond: B -> API : signal\ncoregion API {\nfirst.receive\nsecond.receive\n}\n}\nlayout {\nsecond.receive above first.receive : gap=140\n}\n";
    let diagram = sequence::parse(source).unwrap();
    assert_eq!(diagram.vertical_layout[0].gap, 140.0);
    let svg = sequence::render::render(&diagram, TextMode::System).unwrap();
    assert!(coordinate(&svg, "first", "receive") - coordinate(&svg, "second", "receive") >= 139.99);
    assert!(sequence::parse(&source.replace("gap=140", "gap=-2")).is_err());
    assert!(sequence::parse(&source.replace("gap=140", "gap=NaN")).is_err());
}

#[test]
fn message_send_precedes_receive_even_inside_same_coregion() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nfirst: A -> A : ping\nsecond: A -> A : pong\ncoregion A {\nfirst.send\nfirst.receive\nsecond.send\nsecond.receive\n}\n}\nlayout {\nfirst.receive above first.send\n}\n";
    assert!(
        sequence::parse(source)
            .unwrap_err()
            .message
            .contains("contradictory vertical")
    );
}

#[test]
fn mandatory_coregion_gap_pushes_later_lifeline_event_down() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nfirst: B -> A : one\nsecond: B -> A : two\ncoregion A {\nfirst.receive\nsecond.receive\n}\nlater: B -> A : later\n}\nlayout {\nsecond.receive above first.receive : gap=300\n}\n";
    let diagram = sequence::parse(source).unwrap();
    let svg = sequence::render::render(&diagram, TextMode::System).unwrap();
    assert!(coordinate(&svg, "later", "receive") > coordinate(&svg, "first", "receive") + 24.0);
}

#[test]
fn mandatory_vertical_gap_expands_its_fragment_boundary() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nalt \"x\" {\nfirst: B -> A : one\nsecond: B -> A : two\ncoregion A {\nfirst.receive\nsecond.receive\n}\n}\n}\nlayout {\nsecond.receive above first.receive : gap=400\n}\n";
    let diagram = sequence::parse(source).unwrap();
    let svg = sequence::render::render(&diagram, TextMode::System).unwrap();
    let frame = svg
        .lines()
        .find(|line| line.contains("data-fragment-operator=\"alt\""))
        .unwrap();
    let attr = |name: &str| -> f64 {
        frame
            .split(&format!(" {name}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    assert!(attr("y") + attr("height") > coordinate(&svg, "first", "receive") + 8.0);
}

#[test]
fn expanded_gap_moves_following_notes_continuations_and_fragment_boundary() {
    let source = "settings {\ntype: sequence\n}\nsemantics {\nA\nB\nalt \"x\" {\nfirst: B -> A : one\nsecond: B -> A : two\ncoregion A {\nfirst.receive\nsecond.receive\n}\nhint: note \"After both\"\nwait: continuation \"Done\"\n}\n}\nlayout {\nsecond.receive above first.receive : gap=260\nhint over A\nwait spans A,B\n}\n";
    let svg =
        sequence::render::render(&sequence::parse(source).unwrap(), TextMode::System).unwrap();
    let attr = |line: &str, name: &str| -> f64 {
        line.split(&format!(" {name}=\""))
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    let note = svg
        .lines()
        .find(|line| line.contains("data-note=\"hint\""))
        .unwrap();
    let cont = svg
        .lines()
        .find(|line| line.contains("data-continuation=\"wait\""))
        .unwrap();
    let frame = svg
        .lines()
        .find(|line| line.contains("data-fragment-operator=\"alt\""))
        .unwrap();
    let first = coordinate(&svg, "first", "receive");
    let note_y: f64 = note
        .split(" d=\"M")
        .nth(1)
        .unwrap()
        .split(',')
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(note_y > first);
    assert!(attr(cont, "y") > note_y);
    assert!(attr(frame, "y") + attr(frame, "height") > attr(cont, "y"));
}
