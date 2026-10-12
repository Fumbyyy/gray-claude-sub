<p align="center">
  <img src="assets/gray-logo.svg" alt="gray" width="96">
  <img src="assets/claude.svg" alt="claude" width="96">
</p>
<h1 align="center">gray-claude-sub</h1>
<p align="center">Use your Claude Pro/Max subscription as a model provider — the official `claude` CLI, driven fully inert.</p>
<p align="center">
  <a href="https://github.com/vstaln/gray-claude-sub/blob/main/LICENSE"><img alt="MIT License" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="gray plugin" src="https://img.shields.io/badge/gray-plugin-7aa2f7.svg">
  <img alt="rust" src="https://img.shields.io/badge/built%20with-rust-orange.svg">
</p>

`gray-claude-sub` drives the official `claude` CLI fully inert (`--tools ''`, no MCP servers, no slash commands, `dontAsk`, no session persistence) as a request-scoped model provider. Gray owns the agent loop, tools, approvals, and compaction — Claude only answers.

---

## Highlights

* **No Credential Reads**: The plugin never opens your Claude credential file or reads your OAuth token. Model discovery is a plain GET of Anthropic's public models page, with no login. Chat and usage run the official `claude` CLI, which uses your own `claude auth login`. The only bearer the plugin creates is a per-turn token for its own loopback relay.
* **Single-Request Admission Relay**: Drives an internal request-scoped loopback admission relay that enforces exactly one provider request per turn and absorbs redundant recovery attempts.
* **Native Assistant Replay Carrier**: Assistant messages round-trip in signed reasoning replay carriers (`NATIVE_ITEM_ID`) so future turns restore byte-identical provider frames without prompt divergence.
* **Model Catalog**: Routes and context windows come from Anthropic's public models page, cached on disk. Nothing is compiled in.

---

## Requirements

| Requirement | Details |
|---|---|
| **Claude Code CLI** | Official `@anthropic-ai/claude-code` CLI installed and on `$PATH`. |
| **Authentication** | Logged in via `claude auth login` on your Claude Pro or Max subscription. |
| **Gray Harness** | [Gray](https://github.com/vstaln/gray) agent framework (Protocol v1.1 or v1.2 sidecar support). |

The plugin validates dependencies at every seam:
* If `claude` is not found, it reports an informative installation hint rather than a process crash.
* If custom `ANTHROPIC_BASE_URL` or conflicting cloud backend variables are set, it fails closed to prevent leaking subscription authentication.

---

## Installation & Build

### Building from Source

```sh
git clone https://github.com/vstaln/gray-claude-sub.git
cd gray-claude-sub
cargo build --release
```

The resulting binary is located at `./target/release/claude-sub`.

### Installing in Gray

Point Gray to the binary or install it directly into `$GRAY_HOME/plugins` (default `~/.gray/plugins`):

```sh
# Copy binary to your Gray plugin directory
mkdir -p ~/.gray/plugins/claude-sub
cp target/release/claude-sub ~/.gray/plugins/claude-sub/claude-sub

# Or install via gray plugin management
gray install plugin claude-sub
```

Authenticate with Claude Code if you haven't already:

```sh
claude auth login
```

---

## Model Selection

Once registered, Claude subscription models appear under the `claude-sub/` prefix:

```sh
# Select model in interactive mode
/model claude-sub/sonnet
/model claude-sub/opus
/model claude-sub/haiku
/model claude-sub/fable

# Or launch directly with Gray CLI
gray -m claude-sub/sonnet -p "Review this codebase"
```

### Model Catalog

Routes and context windows come from Anthropic's public models page, `https://platform.claude.com/docs/en/models/overview.md`, read with no credentials. Each successful parse is cached in `$GRAY_HOME/claude-sub/models-cache.json` (default `~/.gray/claude-sub/models-cache.json`) and answers when the page can't be fetched or parsed. With no cache, the model list says why it can't be listed.

The page gives no context window for legacy models, so they get no `[1m]` route. It states no effort levels, so no model offers reasoning-effort levels. Set `CLAUDE_SUB_MODELS_URL` to read the page from another URL.

---

## Wire Protocol & Architecture

```
┌──────────────────┐               ┌──────────────────┐               ┌───────────────────┐
│                  │  stdio NDJSON │                  │  stdin/stdout │                   │
│   Gray Harness   │ ────────────> │ gray-claude-sub  │ ────────────> │  claude CLI       │
│   (owns tools &  │ <──────────── │ (admission relay │ <──────────── │  (inert runner)   │
│   approvals)     │ Protocol 1.2  │ & frame encoder) │  stream-json  │                   │
└──────────────────┘               └──────────────────┘               └───────────────────┘
```

The sidecar communicates over standard I/O using newline-delimited JSON (NDJSON):
* `plugin/manifest`: Declares provider capabilities and supported model IDs.
* `provider/models`: Returns the catalog from the public models page, or the cached one.
* `provider/chat`: Parks the request intent and initiates a relayed turn.
* `provider/auth/*`: Reports external CLI authentication state.

---

## Repository Structure

```text
├── src/
│   ├── main.rs              # Protocol-1.2 sidecar entry point
│   ├── chat.rs              # Request translation, streaming parser, and CLI runner
│   ├── relay.rs             # Single-admission loopback relay proxy
│   ├── catalog.rs           # Model catalog and disk cache
│   ├── models.rs            # Public models-page fetch and parser
│   ├── manifest.rs          # Plugin protocol manifest
│   ├── setup.rs             # CLI dependency discovery and verification
└── reference/               # DirectSDK test & eval suite
```

---

## License

MIT License — Copyright (c) 2026 Vstalin Grady

---
Part of the [gray](https://github.com/vstaln/gray) plugin ecosystem —
the open-source AI agent harness. <https://gray.alignment.id>
