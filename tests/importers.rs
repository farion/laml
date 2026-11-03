use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn directory() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "laml-import-test-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&dir).unwrap();
    dir
}
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_laml"))
        .args(args)
        .env("LAML_NO_UPDATE_CHECK", "1")
        .output()
        .unwrap()
}
fn path(path: &Path) -> &str {
    path.to_str().unwrap()
}
fn import_and_build(extension: &str, source: &str, expected: &[&str]) {
    let dir = directory();
    let input = dir.join(format!("source.{extension}"));
    fs::write(&input, source).unwrap();
    let result = cli(&["import", path(&input)]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for name in expected {
        let file = if expected.len() == 1 {
            input.with_extension("laml")
        } else {
            dir.join("source-laml").join(format!("{name}.laml"))
        };
        assert!(file.is_file(), "missing {}", file.display());
        let image = file.with_extension("svg");
        let built = cli(&["build", path(&file), path(&image)]);
        assert!(
            built.status.success(),
            "{}: {}",
            file.display(),
            String::from_utf8_lossy(&built.stderr)
        );
        assert!(fs::read_to_string(&image).unwrap().contains("<svg "));
    }
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn mermaid_graph_class_and_sequence_build() {
    import_and_build(
        "mmd",
        "flowchart LR\n Start[Request] -->|submit| API[Service]\n API --> Decision{Approved}\n",
        &["diagram"],
    );
    import_and_build(
        "mmd",
        "flowchart LR\n Client\n subgraph Backend[Services]\n API[API]\n DB[Database]\n API --> DB\n end\n Client --> API\n",
        &["diagram"],
    );
    import_and_build(
        "mmd",
        "classDiagram\nclass Animal {\n  +name: String\n}\nclass Dog\nAnimal <|-- Dog\n",
        &["diagram"],
    );
    import_and_build(
        "mmd",
        "sequenceDiagram\nparticipant Alice as Customer\nparticipant Bob as API\nAlice->>Bob: Request\nBob-->>Alice: Response\n",
        &["diagram"],
    );
}
#[test]
fn plantuml_class_and_sequence_build() {
    import_and_build(
        "puml",
        "@startuml\nclass \"Customer account\" as Account {\n+balance: int\n}\nclass Bank\nBank *-- Account : owns\n@enduml\n",
        &["diagram"],
    );
    import_and_build(
        "puml",
        "@startuml\npackage Billing {\n class Invoice\n class Line\n Invoice -- Line\n}\nclass Customer\nCustomer --> Invoice\n@enduml\n",
        &["diagram"],
    );
    import_and_build(
        "puml",
        "@startuml\nparticipant \"Customer\" as Alice\nparticipant API\nAlice -> API: Request\nAPI --> Alice: Response\n@enduml\n",
        &["diagram"],
    );
}

const DSL: &str = "workspace \"Bank\" {\n    model {\n        customer = person \"Customer\" \"Views balances\"\n        bank = softwareSystem \"Bank\" \"Online banking\" {\n            api = container \"API\" \"Banking API\" \"Java\" {\n                service = component \"Service\" \"Bank service\" \"Spring\"\n            }\n        }\n        customer -> bank \"Uses\"\n        customer -> api \"Calls\"\n        api -> service \"Dispatches\"\n    }\n    views {\n        systemContext bank Context {\n            include *\n            autoLayout lr\n        }\n        container bank Containers {\n            include customer\n            include api\n        }\n        component api Components {\n            include customer\n            include service\n        }\n    }\n}\n";
const JSON: &str = r#"{"model":{"people":[{"id":"1","name":"Customer","description":"Views balances","relationships":[{"id":"10","sourceId":"1","destinationId":"2","description":"Uses"},{"id":"11","sourceId":"1","destinationId":"3","description":"Calls"}]}],"softwareSystems":[{"id":"2","name":"Bank","description":"Online banking","containers":[{"id":"3","name":"API","description":"Banking API","technology":"Java","components":[{"id":"4","name":"Service","description":"Bank service","technology":"Spring"}],"relationships":[{"id":"12","sourceId":"3","destinationId":"4","description":"Dispatches"}]}]}]},"views":{"systemContextViews":[{"key":"Context","softwareSystemId":"2","elements":[{"id":"1"},{"id":"2"}],"relationships":[{"id":"10"}]}],"containerViews":[{"key":"Containers","softwareSystemId":"2","elements":[{"id":"1"},{"id":"3"}],"relationships":[{"id":"11"}]}],"componentViews":[{"key":"Components","containerId":"3","elements":[{"id":"1"},{"id":"4"}],"relationships":[{"id":"12"}]}]}}"#;
#[test]
fn structurizr_dsl_and_json_produce_three_buildable_views() {
    for (extension, source) in [("dsl", DSL), ("json", JSON)] {
        import_and_build(extension, source, &["Context", "Containers", "Components"]);
    }
}
#[test]
fn structurizr_nested_this_relationship_builds() {
    let source = "workspace \"X\" {\nmodel {\ns = softwareSystem \"System\" \"Description\"\np = person \"Person\" \"Description\" {\nthis -> s \"Uses\"\n}\n}\nviews {\nsystemContext s Context {\ninclude *\n}\n}\n}\n";
    import_and_build("dsl", source, &["Context"]);
}

#[test]
fn structurizr_rejects_lost_view_elements_and_custom_styles() {
    let missing = JSON.replace(
        "\"id\":\"4\"}],\"relationships\"",
        "\"id\":\"missing\"}],\"relationships\"",
    );
    let styles=JSON.replace("\"views\":{", "\"views\":{\"configuration\":{\"styles\":{\"elements\":[{\"tag\":\"Element\",\"background\":\"#ffffff\"}]}},");
    for (source, needle) in [
        (&missing[..], "unknown model element"),
        (&styles[..], "custom visual styles"),
    ] {
        let dir = directory();
        let input = dir.join("views.json");
        fs::write(&input, source).unwrap();
        let result = cli(&["import", path(&input)]);
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(needle),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!dir.join("views-laml").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn input_is_preserved_and_existing_output_requires_force() {
    let dir = directory();
    let input = dir.join("diagram.mmd");
    fs::write(&input, "flowchart LR\nA --> B\n").unwrap();
    assert!(cli(&["import", path(&input)]).status.success());
    let output = input.with_extension("laml");
    fs::write(&output, "protected").unwrap();
    let refused = cli(&["import", path(&input)]);
    assert!(!refused.status.success());
    assert_eq!(fs::read_to_string(&output).unwrap(), "protected");
    assert!(cli(&["import", path(&input), "--force"]).status.success());
    assert_eq!(
        fs::read_to_string(&input).unwrap(),
        "flowchart LR\nA --> B\n"
    );
    fs::remove_dir_all(dir).unwrap();
}
#[test]
fn unknown_constructs_report_line_and_write_nothing() {
    for (extension, source) in [
        ("mmd", "flowchart LR\nA -.-> B\n"),
        ("mmd", "flowchart LR\nA[(Database)] --> B\n"),
        (
            "puml",
            "@startuml\n!include hidden.puml\nA -> B: msg\n@enduml\n",
        ),
        ("dsl", "workspace \"X\" {\n!include remote.dsl\n}\n"),
    ] {
        let dir = directory();
        let input = dir.join(format!("bad.{extension}"));
        fs::write(&input, source).unwrap();
        let result = cli(&["import", path(&input)]);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("line "));
        assert!(!input.with_extension("laml").exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
