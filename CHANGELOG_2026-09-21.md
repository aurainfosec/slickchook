# Changelog — 2026-09-21

- Added a new `SKILL.md` §6, "Building a visual attack-surface map", per
  direct request, after two real proof-of-concept builds this session:
  one against scanme.nmap.org (Nmap's own public test target) and one
  against aurainfosec.com's real recorded run history (69 runs across
  17 plugins). Both were built entirely from `list_runs_by_target` /
  `list_run_artifacts` / `read_artifact` - the same calls §5 already
  documents - stitched into a domain → IP/subdomain → port/service →
  web-finding tree and published as an HTML Artifact with inline SVG,
  no new tool or code required.
  Codifies what both builds converged on as real, load-bearing
  practice, not just a one-off styling choice:
  - Four consistent node/edge categories - confirmed, tested-negative/
    historical, notable/lead, and flagged-needs-reverification - so the
    diagram distinguishes "checked and found nothing" from "never
    checked", and "unusual, worth a look" from "actually a problem".
  - Call out corroborated findings (multiple independent plugins
    landing on the same underlying fact - e.g. the aurainfosec.com
    build's Cloudflare Bot Management signal, confirmed separately by
    httpx, kiterunner, and gowitness) explicitly as one finding, since
    that's stronger evidence than any single tool's result and explains
    why other tools may have come back empty.
  - Always include an evidence table under the diagram mapping every
    node back to its source plugin/run_id/date - what makes this
    reusable as part of a real report rather than a one-off picture -
    plus a caveats note naming any tool that failed outright rather
    than silently treating failure as a clean result.
  Scoped as an on-request reporting output, not a default end-of-
  engagement deliverable, matching this section's existing archive/
  export outputs.
  Placed in this project's own `SKILL.md` (the recon methodology
  loaded into every MCP client per `CLIENT_SETUP.md`), not the
  platform's bundled artifact-design/artifact-diagramming skills -
  those are general design references this project doesn't own or
  edit; this is project-specific methodology.
  Documentation-only change, no code. `cargo test --lib` still 12/12
  (SKILL.md isn't compiled or exercised by any test).
