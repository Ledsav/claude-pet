# Claude Pet

A tiny desktop pet (Mochi) that lives in a transparent, frameless, always-on-top
window and chats through your local Claude Code CLI.

- **Drag** the pet anywhere; **click** it for a wiggle.
- **Hover** to see food / mood / energy and the **feed**, **play**, **sleep** buttons and a chat box.
- **Tray icon**: left-click shows/hides the pet; right-click for feed, play, sleep,
  *Edit personality…* and quit. The tooltip shows current stats.

## How it works

| Piece | Where |
| --- | --- |
| Stats, decay, actions | `src-tauri/src/stats.rs` – ticks every 5 s, catches up on time the app was closed |
| Save file | `%APPDATA%\dev.claudepet.desktop\pet.json` (written atomically) |
| Chat | `src-tauri/src/chat.rs` – runs `claude -p "<prompt>"` |
| Personality | `%APPDATA%\dev.claudepet.desktop\brain\CLAUDE.md`, seeded from `src-tauri/personality/CLAUDE.md` |

Chat runs `claude` with its working directory set to the `brain` folder, so Claude Code
loads that folder's `CLAUDE.md` as the pet's personality — edit it to change who
Mochi is. Each prompt carries the current stats and the last 6 exchanges. The CLI
runs with all tools disabled (`--tools ""`), no MCP servers, no session
persistence and without your user-level settings/hooks, so the pet can only talk.

Rate limits: one message in flight, at least 4 s apart, at most 30 per hour, 90 s timeout.

Environment overrides: `CLAUDE_PET_MODEL` (default `haiku`), `CLAUDE_PET_CLI`
(path to the `claude` binary; defaults to `~/.local/bin/claude.exe`, then `PATH`).

## Develop

Needs Node 20+, Rust (MSVC toolchain on Windows) and a logged-in `claude` CLI.

```bash
npm install
npm run tauri dev
```

```bash
cd src-tauri && cargo test
```

`cargo test -- --ignored` also runs a live test against the real `claude` CLI.
`npm run tauri build` produces installers in `src-tauri/target/release/bundle`.
