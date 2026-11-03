//! Local MCP bridge: exposes the installed CLI and bundled language guidance.
//! stdout is reserved for MCP; the compiler runs as a child process.

use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ContentBlock},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::Deserialize;
use std::process::Command;

#[derive(Clone)]
struct LamlMcp;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct Topic {
    /// Diagram type (e.g. gne, class, sequence, bpmn, c4-context) or "getting-started".
    topic: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct Source {
    /// Path to a .laml file in the current workspace. Relative paths use the agent's project directory.
    file: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct Build {
    /// Path to an existing .laml source file in the current workspace.
    file: String,
    /// Optional output file ending in .svg, .png or .pdf; defaults to an SVG beside the source.
    output: Option<String>,
}

const TYPES: &str = "LAML compiles declarative .laml sources into SVG, PNG or vector PDF. Supported diagram types: gne (generic node-edge, the default), class, object, component, deployment, state, activity, usecase, package, sequence, communication, interaction-overview, timing, composite, profile, bpmn, c4-context, c4-container, c4-component, c4-code. Source sections separate settings, semantics, layout (hard constraints) and styling (LSS). Use laml_guide(topic) for syntax and laml_example(topic) for a complete, buildable source; write a .laml file in the workspace, then laml_build it. laml_check is a quick parse/style check; only laml_build checks all measured geometry and routing. Rendering can overwrite the selected output file.";

fn example(topic: &str) -> Option<&'static str> {
    Some(match topic {
        "gne" | "generic" => include_str!("../examples/gne-basic.laml"),
        "class" => include_str!("../examples/class-quickstart.laml"),
        "object" => include_str!("../examples/object-basic.laml"),
        "component" => include_str!("../examples/component-quickstart.laml"),
        "deployment" => include_str!("../examples/deployment-quickstart.laml"),
        "state" => include_str!("../examples/state-quickstart.laml"),
        "activity" => include_str!("../examples/activity-basic.laml"),
        "usecase" | "use-case" => include_str!("../examples/usecase-quickstart.laml"),
        "package" => include_str!("../examples/package-quickstart.laml"),
        "sequence" => include_str!("../examples/sequence-quickstart.laml"),
        "communication" => include_str!("../examples/communication-quickstart.laml"),
        "interaction-overview" => include_str!("../examples/interaction-overview-basic.laml"),
        "timing" => include_str!("../examples/timing-basic.laml"),
        "composite" => include_str!("../examples/composite-basic.laml"),
        "profile" => include_str!("../examples/profile-quickstart.laml"),
        "bpmn" => include_str!("../examples/bpmn-basic.laml"),
        "c4-context" => include_str!("../examples/c4-banking-context.laml"),
        "c4-container" => include_str!("../examples/c4-banking-container.laml"),
        "c4-component" => include_str!("../examples/c4-banking-component.laml"),
        "c4-code" => include_str!("../examples/c4-banking-code.laml"),
        _ => return None,
    })
}

fn guide(topic: &str) -> Option<&'static str> {
    Some(match topic {
        "getting-started" => include_str!("../docs/getting-started.md"),
        "gne" | "generic" => include_str!("../docs/diagrams/gne.md"),
        "class" => include_str!("../docs/diagrams/class.md"),
        "object" => include_str!("../docs/diagrams/object.md"),
        "component" => include_str!("../docs/diagrams/component.md"),
        "deployment" => include_str!("../docs/diagrams/deployment.md"),
        "state" => include_str!("../docs/diagrams/state.md"),
        "activity" => include_str!("../docs/diagrams/activity.md"),
        "usecase" | "use-case" => include_str!("../docs/diagrams/usecase.md"),
        "package" => include_str!("../docs/diagrams/package.md"),
        "sequence" => include_str!("../docs/diagrams/sequence.md"),
        "communication" => include_str!("../docs/diagrams/communication.md"),
        "interaction-overview" => include_str!("../docs/diagrams/interaction-overview.md"),
        "timing" => include_str!("../docs/diagrams/timing.md"),
        "composite" => include_str!("../docs/diagrams/composite.md"),
        "profile" => include_str!("../docs/diagrams/profile.md"),
        "bpmn" => include_str!("../docs/diagrams/bpmn.md"),
        "c4-context" | "c4-container" | "c4-component" | "c4-code" => {
            include_str!("../docs/c4-diagrams.md")
        }
        _ => return None,
    })
}

fn cli(args: &[&str]) -> CallToolResult {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(err) => return CallToolResult::error(vec![ContentBlock::text(err.to_string())]),
    };
    // Avoid a background network update check on every agent tool call.
    let output = Command::new(exe)
        .args(args)
        .env("LAML_NO_UPDATE_CHECK", "1")
        .output();
    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            let stderr = String::from_utf8_lossy(&result.stderr);
            if result.status.success() {
                CallToolResult::success(vec![ContentBlock::text(stdout.trim().to_string())])
            } else {
                CallToolResult::error(vec![ContentBlock::text(format!(
                    "{}{}{}",
                    stderr.trim(),
                    if stderr.is_empty() || stdout.is_empty() {
                        ""
                    } else {
                        "\n"
                    },
                    stdout.trim()
                ))])
            }
        }
        Err(err) => CallToolResult::error(vec![ContentBlock::text(err.to_string())]),
    }
}

#[tool_router]
impl LamlMcp {
    #[tool(
        description = "Discover LAML's diagram types, compiler capabilities and workflow before authoring a diagram"
    )]
    fn laml_types(&self) -> String {
        TYPES.to_string()
    }

    #[tool(
        description = "Read bundled LAML documentation for a diagram type; use laml_example for a complete, buildable source. Call with a diagram type from laml_types, or getting-started."
    )]
    fn laml_guide(&self, Parameters(Topic { topic }): Parameters<Topic>) -> CallToolResult {
        match guide(&topic.to_ascii_lowercase()) {
            Some(text) => CallToolResult::success(vec![ContentBlock::text(text.to_string())]),
            None => CallToolResult::error(vec![ContentBlock::text(format!(
                "Unknown topic: {topic}. Call laml_types for supported types."
            ))]),
        }
    }

    #[tool(
        description = "Get a complete, buildable LAML example for a diagram type from laml_types."
    )]
    fn laml_example(&self, Parameters(Topic { topic }): Parameters<Topic>) -> CallToolResult {
        match example(&topic.to_ascii_lowercase()) {
            Some(text) => CallToolResult::success(vec![ContentBlock::text(text.to_string())]),
            None => CallToolResult::error(vec![ContentBlock::text(format!(
                "Unknown diagram type: {topic}. Call laml_types for supported types."
            ))]),
        }
    }

    #[tool(
        description = "Quickly check an existing .laml file in the agent's workspace. This does NOT check all measured geometry; use laml_build for a complete validation and image."
    )]
    fn laml_check(&self, Parameters(Source { file }): Parameters<Source>) -> CallToolResult {
        cli(&["check", &file])
    }

    #[tool(
        description = "Render an existing .laml workspace file as SVG, PNG or PDF. Validates full layout and routing. May overwrite the output file; on failure returns the compiler diagnostic. Defaults to an SVG next to the source."
    )]
    fn laml_build(&self, Parameters(Build { file, output }): Parameters<Build>) -> CallToolResult {
        match output {
            Some(output) => cli(&["build", &file, &output]),
            None => cli(&["build", &file]),
        }
    }
}

#[tool_handler(
    name = "laml",
    version = "1.15.6",
    instructions = "LAML is installed locally for architecture, C4, UML and BPMN diagrams. When asked for a diagram, discover the appropriate type with laml_types, read laml_guide for its real syntax and laml_example for complete buildable source, write a .laml file in the current project workspace, then call laml_build to render and fully validate it. The source file is the source of truth. Edit it and rebuild to fix diagnostic errors. If a preview is needed, open the resulting image using your host's file/image tools."
)]
impl ServerHandler for LamlMcp {}

pub fn run() -> Result<(), String> {
    tokio::runtime::Runtime::new()
        .map_err(|e| e.to_string())?
        .block_on(async {
            LamlMcp
                .serve(stdio())
                .await
                .map_err(|e| e.to_string())?
                .waiting()
                .await
                .map(|_| ())
                .map_err(|e| e.to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_diagram_types_have_guides() {
        for kind in [
            "gne",
            "class",
            "object",
            "component",
            "deployment",
            "state",
            "activity",
            "usecase",
            "package",
            "sequence",
            "communication",
            "interaction-overview",
            "timing",
            "composite",
            "profile",
            "bpmn",
            "c4-context",
            "c4-container",
            "c4-component",
            "c4-code",
        ] {
            assert!(guide(kind).is_some(), "missing guide for {kind}");
            assert!(example(kind).is_some(), "missing example for {kind}");
        }
    }
}
