//! Manual debug-build measurements; excluded from ordinary test runs.
use super::*;
fn fixture(count: usize) -> (String, Action, Vec<Target>) {
    let names = ["A", "B", "C", "D", "E", "F"];
    let mut source =
        "settings {\n type: sequence\n}\nsemantics {\n A\n B\n C\n D\n E\n F\n work: B+\n"
            .to_string();
    for i in 0..count {
        source.push_str(&format!(
            " step{i}: {} ->> {}: step {i}\n",
            names[i % 5],
            names[(i + 1) % 5]
        ));
    }
    source.push_str(" B-\n}\n");
    let preview = crate::preview::build(&source, Path::new("benchmark.laml")).unwrap();
    let targets = preview
        .regions
        .iter()
        .filter(|r| r.kind == "sequence-slot")
        .flat_map(|slot| {
            names.iter().map(move |owner| Target {
                dock: Dock {
                    owner: (*owner).into(),
                    before: slot.line,
                    boundary: None,
                },
                same_row: false,
                straighten: false,
            })
        })
        .collect();
    (
        source,
        Action::Create {
            origin: Dock {
                owner: "F".into(),
                before: 12,
                boundary: None,
            },
            operator: "->".into(),
        },
        targets,
    )
}
#[test]
#[ignore = "Manual target-preflight performance measurement"]
fn sequence_target_preflight_measurement() {
    for messages in [12, 36] {
        let (source, action, targets) = fixture(messages);
        let start = std::time::Instant::now();
        let choices = crate::backend::local().sequence_targets(
            &source,
            Path::new("benchmark.laml"),
            Presentation::Document,
            &action,
            &targets,
        );
        eprintln!(
            "messages={messages} targets={} valid={} elapsed_ms={:.1}",
            targets.len(),
            choices.len(),
            start.elapsed().as_secs_f64() * 1000.0
        );
        assert!(!choices.is_empty());
    }
}
#[test]
#[ignore = "Manual source-adapter performance measurement"]
fn sequence_target_adapter_measurement() {
    for messages in [12, 36] {
        let (source, action, targets) = fixture(messages);
        let start = std::time::Instant::now();
        let choices = build_with(&source, &action, &targets, |text| {
            let doc = laml::sequence::parse(text).ok()?;
            let mut regions = Vec::new();
            for e in &doc.events {
                if let laml::sequence::Event::Message(m) = e {
                    regions.push(laml::authoring::Region::rect(
                        &m.from,
                        "sequence-send",
                        m.line,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                    ));
                    regions.push(laml::authoring::Region::rect(
                        &m.to,
                        "sequence-receive",
                        m.line,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                    ));
                }
            }
            Some((doc, regions))
        });
        eprintln!(
            "adapters messages={messages} targets={} choices={} elapsed_ms={:.1}",
            targets.len(),
            choices.len(),
            start.elapsed().as_secs_f64() * 1000.0
        );
    }
}
