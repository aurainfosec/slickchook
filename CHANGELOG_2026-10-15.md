# Changelog — 2026-10-15

- Added a new `ROADMAP.md` section, "Prompt-injection resilience and
  testing", per direct request. Covers two related but distinct
  directions:
  - Defensive: extending hidden/encoded-instruction detection beyond
    screenshots (the existing `screensanitize`/SC-017 pairing) to
    arbitrary text/HTML/JSON content any fetcher or crawler plugin
    returns, plus other content-based injection surfaces a target can
    serve back at the reading AI client (HTTP headers, DNS TXT
    records, file contents, zero-width Unicode, homoglyphs, RTL
    override tricks) - a text-based analog of what `screensanitize`
    already does for images.
  - Offensive/testing: a dedicated capability for testing whether a
    target's own AI/LLM-integrated feature (a chatbot, support agent,
    AI-powered search, etc.) is itself vulnerable to prompt injection -
    an authorized red-team technique distinct from everything this
    fleet currently runs, and from the defensive items above.
  Also added a regression-test item (untrusted asset content must
  never gain tool-calling authority regardless of which plugin or code
  path it arrived through) to make the defensive direction verifiable,
  not just aspirational.
  Bumped the roadmap's own "Updated" date. Documentation-only change,
  no code - `ROADMAP.md` isn't compiled or exercised by any test.
  Verified: `cargo build --locked --release --bins` and `cargo test
  --lib` (12/12) clean.
