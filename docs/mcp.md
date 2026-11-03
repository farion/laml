# Local AI agent integration (MCP)

LAML ships a local MCP server in the same binary as the CLI. It runs on stdio: the agent starts `laml mcp` as a child process; no listening port, account, or extra service is needed. Install a version of LAML that includes the `mcp` command first. An older installed `laml` binary will not acquire MCP support merely because you configure an agent.

The server announces LAML's capabilities and exposes six tools:

- `laml_types`: discover diagram types and the authoring workflow.
- `laml_guide(topic)`: read the bundled guide for `gne`, any UML type, `bpmn`, or one of the four C4 types; `getting-started` covers shared syntax.
- `laml_example(topic)`: get a complete, buildable `.laml` source for any diagram type.
- `laml_relations(topic)`: read the relationship notation appropriate to that diagram type, including the distinction between semantic connections and placement rules.
- `laml_check(file)`: quick source check; it does **not** check all measured geometry.
- `laml_build(file, output?)`: build SVG (default), PNG, or PDF and validate layout/routing; returns errors from the compiler on failure. An explicitly specified output file may be overwritten.

The agent extracts **both elements and their relationships** from your architecture description, chooses a diagram type, consults its relationship notation, writes the `.laml` source in its project workspace using its normal file tools, then calls `laml_build`. If endpoints, directions or relationship meanings are missing from the description, it should not invent them silently. Relative paths resolve against the agent's project directory. The server does not generate source files on the agent's behalf, and it does not preview or inspect the generated image: use the agent's image-reading capability to review it. The server does not start LAML's optional background update check while compiling.

## OpenCode

From the directory of the project where you want the agent to create diagrams:

```sh
opencode mcp add laml -- laml mcp
opencode mcp list
```

This uses project-scoped configuration (run from a Git project so OpenCode can discover the project config). Use `--global` before `--` if you want the tool in every project. Alternatively configure a local server in `opencode.json` with `"mcp": {"laml": {"type": "local", "command": ["laml", "mcp"]}}`. OpenCode 2 uses `mcp.servers` instead of `mcp.laml`; follow the configuration format of the installed OpenCode version.

If `laml` is not in OpenCode's PATH, use an absolute path to the executable after `--`. Confirm that this binary supports `laml mcp` before registering it. From OpenCode, ask: “Use LAML to draw a C4 context diagram. Save the `.laml` source in this workspace and render an SVG.”

## Claude Code

```sh
claude mcp add --transport stdio laml -- laml mcp
claude mcp list
```

The default scope is local to the current project. Use `--scope project` if the team should share the configuration, or `--scope user` for all your projects. As with OpenCode, use an absolute executable path if needed.

## Design and trust boundary

The compiler is the same CLI for agent and human. MCP only supplies discoverability and tool calls; a cloud model used by the agent may receive tool outputs or source contents even though the LAML process itself is local. The tools accept workspace-relative or absolute file paths with the same file access rights as the agent process. Only enable the MCP server for agents you trust with those rights. Compiler errors are reported as MCP tool errors; stdout of the MCP process is reserved for protocol messages.
