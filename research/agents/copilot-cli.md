# GitHub Copilot CLI Research

**Version Tested:** 0.0.401-1

## Overview

The GitHub Copilot CLI is an AI-powered coding assistant that runs in the terminal. It supports both interactive and non-interactive modes, session management, MCP servers, and various permission controls.

## SDK Communication Protocol

### Connection Modes

The Copilot CLI supports two SDK communication modes:

1. **Stdio Mode** (`--server --stdio`): JSON-RPC over stdin/stdout pipes
2. **TCP Mode** (`--server --port <port>`): JSON-RPC over TCP socket

### JSON-RPC Methods

| Method | Description |
|--------|-------------|
| `ping` | Health check with protocol version |
| `status.get` | Get version and protocol info |
| `auth.getStatus` | Get authentication status |
| `models.list` | List available models |
| `session.create` | Create a new session |
| `session.resume` | Resume an existing session |
| `session.destroy` | Destroy a session |
| `session.send` | Send a message to a session |
| `session.abort` | Abort current processing |
| `session.getMessages` | Get session event history |
| `session.getLastId` | Get most recent session ID |
| `session.list` | List all sessions |
| `session.delete` | Delete a session |

### Permission Request Handling

The SDK can register a permission handler that receives requests:

```typescript
interface PermissionRequest {
  kind: "shell" | "write" | "mcp" | "read" | "url";
  toolCallId?: string;
  [key: string]: unknown;
}

interface PermissionRequestResult {
  kind: "approved" | "denied-by-rules" | "denied-no-approval-rule-and-could-not-request-from-user" | "denied-interactively-by-user";
  rules?: unknown[];
}
```

### Custom Tool Registration

SDK sessions can register custom tools that the CLI will invoke:

```typescript
interface Tool {
  name: string;
  description?: string;
  parameters?: ZodSchema | Record<string, unknown>;
  handler: (args: unknown, invocation: ToolInvocation) => Promise<unknown>;
}
```

## CLI Flags

### Core Execution Modes

| Flag | Description |
|------|-------------|
| (no flags) | Start interactive mode |
| `-i, --interactive <prompt>` | Start interactive mode and auto-execute a prompt |
| `-p, --prompt <text>` | Execute prompt in non-interactive mode (exits after) |
| `--acp` | Start as Agent Client Protocol server |
| `--continue` | Resume the most recent session |
| `--resume [sessionId]` | Resume from a previous session (optionally specify ID) |

### Model Selection

| Flag | Description |
|------|-------------|
| `--model <model>` | Set the AI model to use |

**Available models:**
- `claude-sonnet-4.5`
- `claude-haiku-4.5`
- `claude-opus-4.5`
- `claude-sonnet-4`
- `gemini-3-pro-preview`
- `gpt-5.2-codex`
- `gpt-5.2`
- `gpt-5.1-codex-max`
- `gpt-5.1-codex`
- `gpt-5.1`
- `gpt-5`
- `gpt-5.1-codex-mini`
- `gpt-5-mini`
- `gpt-4.1`

### Output & Streaming

| Flag | Description |
|------|-------------|
| `--stream <mode>` | Enable/disable streaming (choices: `on`, `off`) |
| `-s, --silent` | Output only agent response (no stats), useful for scripting with `-p` |
| `--no-color` | Disable all color output |
| `--plain-diff` | Disable rich diff rendering |
| `--banner` | Show the startup banner |
| `--screen-reader` | Enable screen reader optimizations |

### Session Sharing

| Flag | Description |
|------|-------------|
| `--share [path]` | Share session to markdown file (default: `./copilot-session-<id>.md`) |
| `--share-gist` | Share session to a secret GitHub gist |

### Permission Flags

| Flag | Description |
|------|-------------|
| `--allow-all` / `--yolo` | Enable all permissions (tools, paths, URLs) |
| `--allow-all-tools` | Allow all tools without confirmation |
| `--allow-all-paths` | Disable file path verification |
| `--allow-all-urls` | Allow access to all URLs |
| `--allow-tool [tools...]` | Pre-approve specific tools |
| `--deny-tool [tools...]` | Block specific tools |
| `--allow-url [urls...]` | Allow specific URLs/domains |
| `--deny-url [urls...]` | Deny specific URLs/domains |
| `--available-tools [tools...]` | Only these tools will be available |
| `--excluded-tools [tools...]` | These tools will NOT be available |

### Directory & Path Control

| Flag | Description |
|------|-------------|
| `--add-dir <directory>` | Add directory to allowed list (can use multiple times) |
| `--disallow-temp-dir` | Prevent automatic access to system temp directory |
| `--config-dir <directory>` | Set config directory (default: `~/.copilot`) |

### MCP Server Configuration

| Flag | Description |
|------|-------------|
| `--additional-mcp-config <json>` | Additional MCP servers as JSON or file path (prefix with `@`) |
| `--disable-builtin-mcps` | Disable all built-in MCP servers |
| `--disable-mcp-server <name>` | Disable a specific MCP server |
| `--add-github-mcp-tool <tool>` | Add a tool for GitHub MCP server |
| `--add-github-mcp-toolset <toolset>` | Add a toolset for GitHub MCP server |
| `--enable-all-github-mcp-tools` | Enable all GitHub MCP server tools |

### Agent & Custom Instructions

| Flag | Description |
|------|-------------|
| `--agent <agent>` | Specify a custom agent to use |
| `--no-custom-instructions` | Disable loading from AGENTS.md and related files |
| `--no-ask-user` | Disable ask_user tool (autonomous mode) |

### Tool Execution

| Flag | Description |
|------|-------------|
| `--disable-parallel-tools-execution` | Disable parallel tool execution (run sequentially) |

### Logging

| Flag | Description |
|------|-------------|
| `--log-dir <directory>` | Set log file directory (default: `~/.copilot/logs/`) |
| `--log-level <level>` | Log level: `none`, `error`, `warning`, `info`, `debug`, `all`, `default` |

### Other Flags

| Flag | Description |
|------|-------------|
| `-v, --version` | Show version information |
| `-h, --help` | Display help |
| `--experimental` | Enable experimental features |
| `--no-auto-update` | Disable automatic CLI updates |

## Subcommands

| Command | Description |
|---------|-------------|
| `login [--host <host>]` | Authenticate via OAuth device flow |
| `init` | Initialize Copilot instructions for repository |
| `update` | Download the latest version |
| `version` | Display version info and check for updates |
| `plugin` | Manage plugins and marketplaces |
| `help [topic]` | Display help information |

### Plugin Subcommands

```
copilot plugin install <source>
copilot plugin uninstall <name>
copilot plugin update <name>
copilot plugin list
copilot plugin marketplace
```

## Help Topics

| Topic | Description |
|-------|-------------|
| `config` | Configuration settings |
| `commands` | Interactive mode commands |
| `environment` | Environment variables |
| `logging` | Logging configuration |
| `permissions` | Permission system details |

## Interactive Mode Commands

| Command | Description |
|---------|-------------|
| `/add-dir <directory>` | Add directory to allowed list |
| `/agent` | Browse and select available agents |
| `/allow-all`, `/yolo` | Enable all permissions |
| `/clear`, `/new` | Clear conversation history |
| `/compact` | Summarize history to reduce context |
| `/context` | Show context window token usage |
| `/cwd`, `/cd [directory]` | Change/show working directory |
| `/diff` | Review changes in current directory |
| `/exit`, `/quit` | Exit the CLI |
| `/experimental [on\|off]` | Toggle experimental features |
| `/share [file\|gist] [path]` | Share session |
| `/feedback` | Provide feedback |
| `/help` | Show help |
| `/init` | Initialize Copilot instructions |
| `/list-dirs` | Show allowed directories |
| `/login` / `/logout` | Authentication |
| `/mcp [show\|add\|edit\|delete\|disable\|enable]` | Manage MCP servers |
| `/model`, `/models [model]` | Select AI model |
| `/plan [prompt]` | Create implementation plan |
| `/plugin` | Manage plugins |
| `/rename <name>` | Rename current session |
| `/reset-allowed-tools` | Reset allowed tools list |
| `/resume [sessionId]` | Switch to different session |
| `/review [prompt]` | Run code review agent |
| `/session` | Show session info |
| `/skills` | Manage skills |
| `/terminal-setup` | Configure multiline input |
| `/theme` | View/configure theme |
| `/usage` | Display usage metrics |
| `/user` | Manage GitHub user list |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `COPILOT_ALLOW_ALL` | Set to `true` to allow all tools |
| `COPILOT_AUTO_UPDATE` | Set to `false` to disable auto-update |
| `COPILOT_CUSTOM_INSTRUCTIONS_DIRS` | Additional custom instruction directories |
| `COPILOT_MODEL` | Set agent model |
| `COPILOT_GITHUB_TOKEN` | GitHub auth token (highest precedence) |
| `GH_TOKEN` / `GITHUB_TOKEN` | Alternative auth tokens |
| `USE_BUILTIN_RIPGREP` | Set to `false` to use system ripgrep |
| `PLAIN_DIFF` | Set to `true` to disable rich diffs |
| `XDG_CONFIG_HOME` | Override config directory |
| `XDG_STATE_HOME` | Override state directory |
| `COLORFGBG` | Terminal background detection fallback |

## Configuration Settings

Settings stored in config file (default: `~/.copilot/`):

| Setting | Description |
|---------|-------------|
| `allowed_urls` | List of allowed URLs/domains |
| `denied_urls` | List of denied URLs/domains |
| `auto_update` | Auto-update enabled (default: `true`) |
| `banner` | Banner frequency: `always`, `never`, `once` |
| `beep` | Beep on attention needed (default: `true`) |
| `compact_paste` | Collapse large pastes (default: `true`) |
| `custom_agents.default_local_only` | Local-only agents (default: `false`) |
| `experimental` | Experimental features (default: `false`) |
| `launch_messages` | Custom startup messages |
| `log_level` | Log level (default: `default`) |
| `model` | AI model to use |
| `parallel_tool_execution` | Parallel tools (default: `true`) |
| `render_markdown` | Render markdown (default: `true`) |
| `screen_reader` | Screen reader mode (default: `false`) |
| `stream` | Streaming mode (default: `true`) |
| `theme` | Output theme: `auto`, `dark`, `light` |
| `trusted_folders` | Folders with granted permissions |
| `undo_without_confirmation` | Skip undo confirmation (experimental) |
| `update_terminal_title` | Update terminal title (default: `true`) |

## Permission Pattern Syntax

The `--allow-tool` and `--deny-tool` flags use permission patterns:

```
shell(command:*?)      # Shell commands. Use :* for prefix matching
                       # e.g., shell(git:*) matches all git commands

write                  # File creation/modification tools (except shell)

<mcp-server>(tool?)    # MCP server tools
                       # e.g., MyMCP(my_tool) or MyMCP for all

url(domain-or-url?)    # URL access patterns
                       # e.g., url(https://github.com)
```

## Streaming Protocol

Based on the `--stream <mode>` flag with choices `on` or `off`:
- Default behavior is streaming enabled (`stream: true` in config)
- The `--silent` flag suggests text-based output for scripting
- No explicit JSONL or SSE flags visible in CLI help

**Note:** The `--acp` flag indicates Agent Client Protocol support, which may use a different streaming mechanism.

## Session Event Types

Events are stored in `~/.copilot/session-state/{session-uuid}/events.jsonl` as newline-delimited JSON.

### Event Structure

```typescript
interface SessionEvent {
  id: string;           // UUID
  timestamp: string;    // ISO 8601
  parentId: string | null;
  ephemeral?: boolean;  // Not persisted to history
  type: string;
  data: object;
}
```

### Event Types (37 total)

**Session Lifecycle:**
- `session.start` - Session created (includes sessionId, version, copilotVersion, context)
- `session.resume` - Session resumed (includes resumeTime, eventCount)
- `session.error` - Error occurred (includes errorType, message, stack)
- `session.idle` - Session waiting for input (ephemeral)
- `session.info` - Informational message
- `session.model_change` - Model changed
- `session.handoff` - Session handed off between local/remote
- `session.truncation` - Context truncated
- `session.usage_info` - Token usage info (ephemeral)
- `session.compaction_start` / `session.compaction_complete` - Context compaction

**User Messages:**
- `user.message` - User input (includes content, transformedContent, attachments)
- `pending_messages.modified` - Pending queue changed (ephemeral)

**Assistant Messages:**
- `assistant.turn_start` - Turn began (includes turnId)
- `assistant.intent` - Intent update (ephemeral)
- `assistant.reasoning` - Reasoning content
- `assistant.reasoning_delta` - Reasoning streaming chunk (ephemeral)
- `assistant.message` - Final message (includes messageId, content, toolRequests)
- `assistant.message_delta` - Message streaming chunk (ephemeral)
- `assistant.turn_end` - Turn completed
- `assistant.usage` - Token usage for turn (ephemeral)

**Tool Execution:**
- `tool.user_requested` - User manually requested tool
- `tool.execution_start` - Tool started (includes toolCallId, toolName, arguments)
- `tool.execution_partial_result` - Partial output (ephemeral)
- `tool.execution_progress` - Progress update (ephemeral)
- `tool.execution_complete` - Tool finished (includes success, result, error)

**Subagents:**
- `subagent.started` - Subagent spawned
- `subagent.completed` - Subagent finished
- `subagent.failed` - Subagent errored
- `subagent.selected` - Subagent chosen

**System:**
- `system.message` - System prompt content
- `hook.start` / `hook.end` - Hook execution
- `abort` - Abort requested

### Example Events

```json
// Session start
{"type":"session.start","data":{"sessionId":"e40c2baf-...","version":1,"producer":"copilot-agent","copilotVersion":"0.0.401-1","startTime":"2026-02-02T22:07:39.415Z","context":{"cwd":"/home/user/code","gitRoot":"/home/user/code","branch":"main"}}}

// User message
{"type":"user.message","data":{"content":"hello","transformedContent":"<current_datetime>...</current_datetime>\n\nhello","attachments":[]}}

// Assistant turn start
{"type":"assistant.turn_start","data":{"turnId":"0"}}

// Assistant message with tool requests
{"type":"assistant.message","data":{"messageId":"...","content":"","toolRequests":[{"toolCallId":"toolu_...","name":"view","arguments":{"path":"/home/user"},"type":"function"}]}}

// Tool execution
{"type":"tool.execution_start","data":{"toolCallId":"toolu_...","toolName":"view","arguments":{"path":"/home/user"}}}
{"type":"tool.execution_complete","data":{"toolCallId":"toolu_...","success":true,"result":{"content":"file1\nfile2"}}}

// Session idle
{"type":"session.idle","data":{},"ephemeral":true}
```

## Key Observations

1. **Session Management:** Built-in with `--continue` and `--resume [sessionId]`
2. **Working Directory:** Controlled via `--add-dir` and `/cwd` command
3. **Output Formats:**
   - Interactive terminal rendering (default)
   - Silent mode (`-s`) for scripting
   - Markdown export (`--share`)
   - GitHub Gist export (`--share-gist`)
4. **Server Mode:** `--server --stdio` for JSON-RPC over pipes, `--server --port` for TCP
5. **MCP Support:** Native MCP server integration with GitHub MCP built-in
6. **Custom Agents:** Support via `--agent` flag and AGENTS.md files
7. **Skills System:** `/skills` command suggests extensible capabilities

## Example Commands

```bash
# Non-interactive with auto-approve
copilot -p "Fix the bug in main.js" --allow-all

# Resume with specific model
copilot --continue --model claude-sonnet-4

# Debug logging
copilot --log-level debug --log-dir ./logs

# Allow specific git commands
copilot --allow-tool 'shell(git:*)' --deny-tool 'shell(git push)'

# Share session after completion
copilot -p "Refactor utils.js" --allow-all --share ./session.md

# SDK server mode (stdio)
copilot --server --stdio --log-level debug

# SDK server mode (TCP)
copilot --server --port 3000
```

## SDK Integration (copilot-sdk)

The official SDK in `copilot-sdk/nodejs` provides:

### CopilotClient

```typescript
const client = new CopilotClient({
  cliPath: "copilot",    // or path to binary
  useStdio: true,        // stdio (default) or TCP
  port: 0,               // auto-assign port for TCP
  cwd: process.cwd(),    // working directory
  logLevel: "debug",     // none/error/warning/info/debug/all
});

await client.start();
const session = await client.createSession({
  model: "claude-sonnet-4",
  streaming: true,
  tools: [...],          // custom tools
  onPermissionRequest: async (req) => ({ kind: "approved" }),
});
```

### CopilotSession

```typescript
// Subscribe to events
session.on((event) => {
  if (event.type === "assistant.message") {
    console.log(event.data.content);
  }
});

// Send and wait for completion
const response = await session.sendAndWait({ prompt: "Hello!" });

// Get session history
const events = await session.getMessages();

// Clean up
await session.destroy();
await client.stop();
```

## Differences from Other Agents

| Feature | Copilot CLI | Claude Code | Codex |
|---------|-------------|-------------|-------|
| Communication | JSON-RPC (stdio/TCP) | Subprocess JSONL | Shared stdio server |
| Session Resume | Built-in server-side | `--resume` flag | Thread IDs |
| Permissions | Handler callback | Native protocol | N/A |
| Custom Tools | SDK registration | N/A | N/A |
| Multi-turn | Server manages state | CLI manages state | Server manages state |

## Integration Approach for sandbox-agent

Given that Copilot CLI already has a server mode with JSON-RPC, the integration should:

1. **Use `--server --stdio` mode** - Similar to Codex's shared server model
2. **Map JSON-RPC events to universal schema** - Convert session events
3. **Handle permissions via callback** - Route to question/permission events
4. **Leverage session management** - Use session.create/resume/destroy

This is closer to the Codex/OpenCode pattern (shared long-running server) than the Claude/Amp pattern (subprocess per turn).
