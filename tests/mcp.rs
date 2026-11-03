use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn request(stdin: &mut impl Write, stdout: &mut impl BufRead, request: Value) -> Value {
    writeln!(stdin, "{request}").unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[test]
fn stdio_server_discovers_guides_builds_images_and_reports_errors() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_laml"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let init = request(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":"2025-11-25", "capabilities":{},
            "clientInfo":{"name":"test","version":"1"}
        }}),
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "laml");
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("When asked for a diagram")
    );
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc":"2.0","method":"notifications/initialized"})
    )
    .unwrap();
    let tools = request(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    );
    let names: Vec<_> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "laml_build",
            "laml_check",
            "laml_example",
            "laml_guide",
            "laml_types"
        ]
    );
    let example = request(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{
            "name":"laml_example", "arguments":{"topic":"sequence"}
        }}),
    );
    assert!(
        example["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("type: sequence")
    );

    let dir = std::env::temp_dir().join(format!("laml-mcp-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("test.laml");
    let output = dir.join("test.svg");
    std::fs::write(
        &source,
        "semantics {\n    Browser\n    API\n    Browser -> API : request\n}\n",
    )
    .unwrap();
    let built = request(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{
            "name":"laml_build", "arguments":{"file":source,"output":output}
        }}),
    );
    assert_eq!(built["result"]["isError"], false);
    assert!(std::fs::read_to_string(&output).unwrap().contains("<svg"));

    std::fs::write(&source, "semantics {\n    A -> Missing\n}\n").unwrap();
    let failed = request(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{
            "name":"laml_build", "arguments":{"file":source,"output":output}
        }}),
    );
    assert_eq!(failed["result"]["isError"], true);
    assert!(std::fs::read_to_string(&output).unwrap().contains("<svg"));
    drop(stdin);
    assert!(child.wait().unwrap().success());
    std::fs::remove_dir_all(dir).unwrap();
}
