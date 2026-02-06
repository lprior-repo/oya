# OpenCode ↔ OYA Integration Map

This document maps OpenCode TypeScript modules to OYA Rust crates and shows integration points.

## Architecture Alignment

| OpenCode Module | OYA Crate | Status | Notes |
|----------------|-----------|--------|-------|
| `agent/` | `crates/orchestrator/` | ⚠️ Planned | Agent swarm coordination |
| `tool/` | `crates/opencode/` | ⚠️ Planned | Tool execution via opencode CLI |
| `mcp/` | `crates/opencode/` | ⚠️ Planned | MCP server integration |
| `provider/` | `crates/opencode/` | ⚠️ Planned | LLM provider abstraction |
| `session/` | `crates/pipeline/` | ⚠️ Partial | Task persistence, workspace isolation |
| `skill/` | `crates/intent/` | ⚠️ Similar | Intent/KIRK vs OpenCode skills |
| `config/` | `crates/core/` | ✅ Exists | TOML config, env vars |

## Key Integration Points

### 1. OpenCode CLI Execution
**Location**: `crates/opencode/`

Wraps the `opencode` CLI for AI-powered operations:
```rust
// Execute via opencode CLI
let client = OpencodeClient::new()?;
let result = client.execute("Create a function...").await?;
```

**OpenCode Reference**:
- `opencode-main/packages/opencode/src/cli/` - CLI implementation
- `opencode-main/packages/opencode/src/agent/` - Agent logic

### 2. MCP Server Integration
**Location**: `crates/opencode/`

Integrate MCP servers for external tools:
```rust
// MCP server discovery and tool registration
let mcp = McpClient::discover()?;
let tools = mcp.list_tools().await?;
```

**OpenCode Reference**:
- `opencode-main/packages/opencode/src/mcp/client.ts` - MCP protocol
- JSON-RPC 2.0 message format
- Tool/resource registration

### 3. Agent Orchestration
**Location**: `crates/orchestrator/`

Coordinate multiple AI agents in parallel:
```rust
// Spawn parallel agents
let orchestrator = AgentOrchestrator::new();
orchestrator.spawn(agents).await?;
```

**OpenCode Reference**:
- `opencode-main/packages/opencode/src/agent/agent.ts` - Agent implementation
- `opencode-main/packages/opencode/src/acp/` - Agent Communication Protocol

### 4. Skill System (Intent/KIRK)
**Location**: `crates/intent/`

OYA's Intent/KIRK system is similar to OpenCode skills:

| OpenCode Skills | OYA Intent/KIRK |
|----------------|-----------------|
| User-invocable | User-invocable |
| Prompt expansion | Prompt expansion |
| Tool selection | Tool selection |
| Context injection | Context injection |

**OpenCode Reference**:
- `opencode-main/packages/opencode/src/skill/` - Skill system

### 5. Tool Execution
**Location**: `crates/opencode/`

Execute tools via OpenCode's unified interface:
```rust
// Tools: Bash, Read, Write, Edit, Grep, Glob, WebFetch, WebSearch
let executor = ToolExecutor::new();
executor.bash("moon run :test")?;
executor.read("/path/to/file")?;
```

**OpenCode Reference**:
- `opencode-main/packages/opencode/src/tool/bash.ts`
- `opencode-main/packages/opencode/src/tool/read.ts`
- `opencode-main/packages/opencode/src/tool/write.ts`
- `opencode-main/packages/opencode/src/tool/grep.ts`

## OpenCode Features We Use

### ✅ Currently Using
1. **CLI Execution** - Call `opencode run` for AI tasks
2. **Configuration** - `.opencode/config.toml` pattern
3. **Session Management** - Persistent session tracking

### ⚠️ Planned Integration
1. **MCP Servers** - Integrate external tools via MCP protocol
2. **Provider Abstraction** - Multi-LLM support (Anthropic, OpenAI, etc.)
3. **Streaming Responses** - Real-time output streaming
4. **Agent Coordination** - Parallel agent execution
5. **Plugin System** - Extend functionality via plugins

### ❌ Not Using
1. **Desktop App** - CLI/TUI only
2. **Web UI** - Server mode
3. **GitHub Actions** - Using native git/jj
4. **Enterprise Features** - Community edition

## Implementation Patterns

### Error Handling
**OpenCode Pattern** (TypeScript):
```typescript
export type Result<T, E = Error> =
  | { ok: true; value: T }
  | { ok: false; error: E };
```

**OYA Pattern** (Rust):
```rust
pub type Result<T> = std::result::Result<T, Error>;

// Railway-Oriented Programming
value
  .map(|x| process(x))
  .and_then(|x| validate(x))
  .map_err(|e| handle_error(e))?
```

### Configuration
**OpenCode Pattern**:
```toml
# .opencode/config.toml
[model]
name = "claude-sonnet-4.5"

[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
```

**OYA Pattern**:
```toml
# oya.toml
[opencode]
model = "claude-sonnet-4.5"

[opencode.providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
```

### Streaming
**OpenCode Pattern**:
```typescript
for await (const chunk of stream) {
  if (chunk.type === 'content') {
    process.stdout.write(chunk.data);
  }
}
```

**OYA Pattern**:
```rust
while let Some(chunk) = stream.next().await {
  match chunk? {
    ChunkType::Content(data) => print!("{}", data),
    ChunkType::TokenUsage(usage) => track_usage(usage),
  }
}
```

## Reference Code Locations

### For Tool Implementation
- Study: `vendor/opencode-sme/opencode-main/packages/opencode/src/tool/`
- Port to: `crates/opencode/src/tools/`

### For Agent Patterns
- Study: `vendor/opencode-sme/opencode-main/packages/opencode/src/agent/`
- Port to: `crates/orchestrator/src/`

### For MCP Integration
- Study: `vendor/opencode-sme/opencode-main/packages/opencode/src/mcp/`
- Port to: `crates/opencode/src/mcp/`

### For Provider Abstraction
- Study: `vendor/opencode-sme/opencode-main/packages/opencode/src/provider/`
- Port to: `crates/opencode/src/providers/`

## Next Steps

1. **Study OpenCode architecture** in `vendor/opencode-sme/`
2. **Implement MCP client** in `crates/opencode/`
3. **Build provider abstraction** for multi-LLM support
4. **Create agent orchestrator** for parallel execution
5. **Integrate streaming** for real-time feedback

---

**See Also**:
- `vendor/opencode-sme/INDEX.md` - Complete reference library index
- `vendor/opencode-sme/README.md` - OpenCode SME repo overview
- `docs/OYA_ARCHITECTURE.md` - OYA architecture documentation
