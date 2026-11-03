# Local AI agent integration (MCP)

LAML ships a local MCP server in the CLI binary. The agent starts `laml mcp` over stdio and uses the same compiler as the command line.

The server announces LAML's capabilities and exposes six tools:

- `laml_types`: discover diagram types and the authoring workflow.
- `laml_guide(topic)`: read the bundled guide for `gne`, any UML type, `bpmn`, or one of the four C4 types; `getting-started` covers shared syntax.
- `laml_example(topic)`: get a complete, buildable `.laml` source for any diagram type.
- `laml_relations(topic)`: read the relationship notation appropriate to that diagram type, including the distinction between semantic connections and placement rules.
- `laml_check(file)`: check source declarations and references.
- `laml_build(file, output?)`: build SVG (default), PNG, or PDF and validate layout/routing; returns errors from the compiler on failure. An explicitly specified output file may be overwritten.

The agent identifies **elements and relationships** in your architecture description, chooses a diagram type, consults its notation, writes the `.laml` source in its project workspace, and calls `laml_build`. Ask for clarification when endpoints, directions or relationship meanings are ambiguous. Relative paths resolve against the agent's project directory. Review the rendered image with the agent's image-reading capability.

## OpenCode

From the directory of the project where you want the agent to create diagrams:

```sh
opencode mcp add laml -- laml mcp
opencode mcp list
```

Run the command in the project where OpenCode will create diagrams. Use `--global` before `--` to register it across projects.

Use an absolute executable path after `--` when appropriate. From OpenCode, ask: “Use LAML to draw a C4 context diagram. Save the `.laml` source in this workspace and render an SVG.”

## Claude Code

```sh
claude mcp add --transport stdio laml -- laml mcp
claude mcp list
```

The default scope is local to the current project. Use `--scope project` if the team should share the configuration, or `--scope user` for all your projects. As with OpenCode, use an absolute executable path if needed.

## Design and trust boundary

MCP exposes compiler tools to the agent. A cloud model used by the agent may receive source content or tool results. The MCP process has the file access rights of the agent process; configure it for projects whose files that agent may read and write. Compiler errors appear as MCP tool errors; stdout carries protocol messages.
