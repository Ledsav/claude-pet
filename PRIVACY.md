# Privacy policy

Claude Pet has no servers, accounts, analytics or telemetry. The developer receives
nothing from your copy of the app.

## What stays on your computer

The pet's stats, your settings and its personality file are saved in
`%APPDATA%\dev.claudepet.desktop\`. The last few chat exchanges are kept in memory only
and are forgotten when the app quits; Claude Code is told not to save the conversation
either.

## What is sent, and to whom

The app talks to Claude only through the [Claude Code](https://claude.com/claude-code)
CLI that you installed and logged in to yourself. It sends a request when:

- you send a chat message, or
- a pep talk is due (every 45 minutes by default; turn it off with `/settings motivate off`).

Each request contains the pet's name and personality, its current food, mood and energy,
the last few chat exchanges and your new message. Claude Code sends it to Anthropic under
your own account, so [Anthropic's privacy policy](https://www.anthropic.com/legal/privacy)
applies to it. Claude Code runs with all tools disabled, so it cannot read or change your
files.

Nothing is sent to any other system.

## Removing your data

Uninstall the app from *Settings → Apps → Installed apps → Claude Pet*, then delete
`%APPDATA%\dev.claudepet.desktop\` to remove the pet's saved data.
