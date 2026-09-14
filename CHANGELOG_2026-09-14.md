# Changelog — 2026-09-14

- Added concrete example paths to `CLIENT_SETUP.md` §4 for where to load
  `SKILL.md`'s content into a handful of common MCP clients, per direct
  request. The existing guidance (copy it into "whatever mechanism that
  client offers") was correct but left every client to figure out where
  that actually is - added: Claude Code (`~/.claude/CLAUDE.md` user-level,
  or a project-level `CLAUDE.md`), Cursor (`.cursor/rules/*.mdc` with
  `alwaysApply: true`, or the legacy `.cursorrules`), Windsurf
  (`.windsurfrules`), and GitHub Copilot Chat
  (`.github/copilot-instructions.md`). Also called out, explicitly, the
  two common clients where there is no file path at all - Claude Desktop /
  claude.ai Projects (a "Custom instructions" UI text field, not a file;
  `claude_desktop_config.json` only registers MCP servers, it's unrelated)
  and a custom client built directly on the Claude API (the `system`
  parameter of a Messages API call) - so this doesn't get treated as a
  gap in the doc for those clients.
  Documentation-only change, no code.
