# Architecture

This document collects the ASCII diagrams describing Slickchook's intended
deployment architecture, its internal privilege boundaries, how an MCP
client connects to it, and the recon workflow its `SKILL.md` methodology
drives plugins through. These are descriptive diagrams for orientation, not
a spec - the authoritative behavior is always the code and the plugin
manifests themselves.

## Deployment architecture and privilege boundaries

The MCP client and the slickchook host are separate trust domains: the
client runs in its own container and reaches slickchook only over the
network, through a single TLS-terminating front-end. Inside the host,
privilege is deliberately reduced at every hop from the daemon down to the
container a plugin actually runs in.

```
================================================================================
 MCP CLIENT CONTAINER  (e.g. Claude - a container, not a bare host process)
================================================================================
                                            |
                                            |  MCP over TLS (Caddy front-end, :8443)
                                            v
================================================================================
 SLICKCHOOK HOST - UBUNTU SERVER VM  (a full VM, not a container - runs
 dockerd, Caddy, Postgres, and the slickchook daemon directly on the OS)
================================================================================

  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .
  : PRIVILEGE BOUNDARY: unprivileged "slickchook" service account            :
  :                                                                          :
  :   slickchook daemon                                                     :
  :      - terminates the MCP/SSE session from Caddy                        :
  :      - holds no docker-group membership, no socket access itself        :
  :      - on each plugin call, execs slickchook-runner as a subprocess     :
  . . . . . . . . . . . . . . . . . . . |. . . . . . . . . . . . . . . . . . .
                                        v
  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .
  : PRIVILEGE BOUNDARY: slickchook-runner (setgid: docker group)            :
  :                                                                          :
  :   slickchook-runner                                                     :
  :      - the ONLY part of slickchook with docker.sock access              :
  :      - re-validates manifest/artifact/input paths independently         :
  :        of the daemon (defense in depth, not just trusting its caller)   :
  :      - applies each plugin's [docker] policy before spawning it         :
  . . . . . . . . . . . . . . . . . . . |. . . . . . . . . . . . . . . . . . .
                                        v
  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .
  : PRIVILEGE BOUNDARY: Docker Engine (dockerd, root - separate from        :
  : slickchook's own process tree entirely, reached only via docker.sock)   :
  :                                                                          :
  :   Docker Engine                                                         :
  :      |                                                                  :
  :      v                                                                  :
  :   . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .    :
  :   : PRIVILEGE BOUNDARY: per-run plugin container                   :    :
  :   :   [ plugin container ]                                         :    :
  :   :      --rm  --read-only  UID 65534 (nobody)  --cap-drop ALL     :    :
  :   :      no-new-privileges, tmpfs-only writable areas              :    :
  :   :      nmap, nuclei, sqlmap, dalfox, commix, theharvester, ...   :    :
  :   :      (one at a time per run_id; torn down on exit)             :    :
  :   . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .    :
  . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . . .

  PostgreSQL (plugin_runs table)          <- written back to by the runner,
  local artifact store                       outside the container boundary
  (/var/lib/slickchook/artifacts/<run_id>/)

--------------------------------------------------------------------------------
  Roadmap (proposed, not yet built): an OpenVAS/GVM thin-client plugin talking
  over GMP to a separately managed GVM stack elsewhere in the hypervisor -
  out of scope for this diagram.
--------------------------------------------------------------------------------
```

## Client AI container to slickchook MCP interface

How an MCP client actually establishes and uses its connection: an SSE
event stream for the session, JSON-RPC calls tagged to it, and responses
(including from long-running plugin invocations) streaming back
asynchronously over that same channel rather than as a call's own response
body.

```
================================================================================
 CLIENT AI CONTAINER                          (generic - any MCP-speaking agent
                                                runtime: Claude Code, a hosted
                                                agent, etc. - not slickchook-
                                                specific, and not necessarily on
                                                the same host or VM)
================================================================================

  MCP client
     - holds the tool schema (list_plugins, run_plugin, start_plugin /
       plugin_status / plugin_cancel, list_runs_by_target,
       purge_runs_by_target, list_run_artifacts / read_artifact, ...)
     - has no direct network path to Docker, the plugin containers, or
       Postgres - the MCP interface is the ENTIRE surface it can reach
     |
     |  (1) GET /sse         -- opens a long-lived SSE event stream,
     |                           server assigns this connection a session id
     |
     |  (2) POST JSON-RPC    -- tools/list, then tools/call for each
     |                           invocation, tagged with that session id
     |
     |  (3) responses stream back asynchronously over the SAME SSE
     |      connection opened in (1), not as the POST's own response body -
     |      this is why a long-running plugin (start_plugin) can return
     |      immediately while its eventual result arrives later on the
     |      same stream
     v
================================================================================
 NETWORK BOUNDARY  (TLS - the client container is only ever a network peer,
 never a co-tenant of the slickchook host's process/privilege tree)
================================================================================
     |
     v
================================================================================
 SLICKCHOOK HOST - UBUNTU SERVER VM  (a full VM, not a container - runs
 dockerd, Caddy, Postgres, and the slickchook daemon directly on the OS)
================================================================================

  Caddy (front-end, :8443)
     - terminates TLS, reverse-proxies the SSE + JSON-RPC traffic through
     - this is the ONLY externally-reachable port on the host
     |
     v
  slickchook daemon (MCP server, SSE transport)
     - maps each JSON-RPC tool call onto the matching internal method
       (plugin_manager::run_plugin, ::start_plugin, ::read_artifact, ...)
     - this is the same "unprivileged service account" tier from the
       privilege-boundary diagram above - everything below it
       (slickchook-runner, Docker Engine, the per-run plugin container) is
       unchanged from that diagram and omitted here for focus
     v
  ... slickchook-runner -> Docker Engine -> [ per-run plugin container ] ...
```

## Recon workflow

The phased methodology `SKILL.md` drives an MCP client through, from
scope-discipline gating through passive enumeration, active discovery,
web-surface mapping, and assessment, to final findings summary and full
artifact download.

```
  §0  SCOPE DISCIPLINE (gates, not a phase - checked continuously)
      -----------------------------------------------------------
      Confirm explicit authorization before:
        1. any live authenticated/credentialed action against a real service
        2. any query against a shared/internet-wide/third-party/cost-metered
           database (breach-data lookups, paid search APIs, etc.)
        3. any technique whose detection method itself sends deliberately
           attack-shaped payloads (WAF fingerprinting, DAST fuzzing, etc.)
      |
      v
  PHASE 1 - PASSIVE ENUMERATION  (parallel, no contact with target infra)
      passive subdomain/cert/OSINT sources, code-search dorking,
      web-archive/historical crawl, SaaS vanity-domain checks
        |
        +-- optional: permutation generation on discovered names --+
      |                                                             |
      v                                                             v
  PHASE 2 - ACTIVE RESOLUTION / HOST DISCOVERY  (parallel per host) <--+
      DNS resolution of Phase 1 + permuted names, port/service scan
      |
      v
  PHASE 3 - WEB SURFACE DISCOVERY  (parallel per live HTTP(S) host)
      |
      +--> CMS fingerprinting
      +--> API / GraphQL surface discovery
      +--> generic content/path discovery
      |
      v
  PHASE 4 - ASSESSMENT  (parallel per target)
      |
      +--> template-based vuln scanner  +  TLS/cert posture checker (together)
      +--> email security posture (SPF/DKIM/DMARC) - passive
      +--> WAF/CDN fingerprint  -- §0 cat.3 (attack-shaped, low-noise)
      +--> cloud storage bucket enumeration
      +--> identified OSS + version --> dependency manifest fetch
      |        --> offline dependency-vulnerability scanner
      |        --> (separately) check its public issue tracker
      +--> live fingerprint --> CVE/PoC research plugin
      |        (covers cases the manifest-driven path above can't reach)
      +--> JS files found --> source-map plugin
      |        --> chain into filesystem/secret scanner if its own
      |            analysis alone is only lightweight
      +--> JWT-looking string found --> JWT decode/analysis plugin
      |        (using crafted output against a live target is a
      |         separate, deliberate, human-authorized step)
      +--> active fuzzing: SQLi / XSS / SSRF / SSTI / command injection
               -- §0 cat.3: default to quietest setting, escalate only
                  deliberately, detection-only, findings are a
                  human-directed decision point, not auto-escalated
      |
      v
  PHASE 5 - OPT-IN, HIGHER-SENSITIVITY EXTRAS
      screenshot capture (+ sanitization) of discovered web surface
      wordlist generation --> credential-testing chaining (human-gated)
      |
      v
  §4  SUMMARIZE FINDINGS AND RECOMMENDED ACTIONS
      |
      v
  §5  ARTIFACT COLLECTION FOR IN-SCOPE TARGETS
      full archive of every run's artifacts, retained for reference
      + URL-export step: normalize discovered URLs for import into
        Burp (Target > Scope > Advanced scope control > Load) or
        similar external tooling
      |
      v
  §6  VISUAL ATTACK-SURFACE MAP  (on-request reporting output, not a
      default end-of-engagement step)
      gather structural data via the same list_runs_by_target /
      list_run_artifacts / read_artifact calls §5 uses, structure it as
      a tree (target -> resolved hosts -> ports/services -> web-layer
      findings), classify each node confirmed / tested-negative /
      notable / flagged, call out corroborated multi-plugin findings,
      and publish it as an HTML Artifact (inline SVG diagram + an
      evidence table mapping every node back to its source run)
```

### Convergence into summary and download

A higher-altitude view of the same flow, collapsing each phase's internal
branching down to emphasize how everything accumulated across Phases 1-5
feeds into the final synthesis and raw-artifact download:

```
   PHASE 1              PHASE 2                PHASE 3              PHASE 4
   Passive              Active resolution /    Web surface          Assessment
   enumeration          host discovery         discovery            (vuln scan, TLS,
   (parallel,           (parallel per          (parallel per        dependency/CVE
   no target-infra      discovered host)       live HTTP(S) host)   research, JWT,
   contact)                                                         WAF/CDN, cloud
      |                     |                      |                storage, active
      |                     |                      |                fuzzing, ...)
      v                     v                      v                      |
   +-----------------------------------------------------------------+   |
   |            every run's output persists as it happens:           |<--+
   |     PostgreSQL row (plugin_runs) + artifact files on disk,       |
   |         addressable by run_id for the rest of this flow          |
   +-----------------------------------------------------------------+
                                  |
                                  v
                       PHASE 5 (opt-in, higher sensitivity)
                    screenshots, wordlist -> credential testing
                                  |
                                  v
   =============================================================================
    §4  SUMMARIZE FINDINGS AND RECOMMENDED ACTIONS
        - pull together what every phase above actually found
        - call out what's worth a human's attention first, and why
        - this is a synthesis step, not a new tool run - it reads back
          across every run_id accumulated so far for this target
   =============================================================================
                                  |
                                  v
   =============================================================================
    §5  ARTIFACT COLLECTION / DOWNLOAD  (for every in-scope target)
        - list_run_artifacts + read_artifact walk every run_id from every
          phase above and pull its raw tool output back to the client
        - nothing is summarized or filtered here - it's the full,
          unmodified artefact set (logs, JSON, session files, screenshots)
          each plugin actually wrote under /artifacts, retained for
          reference and hand-off
        - URL-export step (separate from the raw archive): normalizes
          discovered URLs for import into Burp (Target > Scope >
          Advanced scope control > Load) or similar external tooling
   =============================================================================
                                  |
                                  v
   =============================================================================
    §6  VISUAL ATTACK-SURFACE MAP  (on request, not a default step)
        - reuses §5's same list_runs_by_target / list_run_artifacts /
          read_artifact calls - no dedicated tool of its own
        - structures findings as a tree and classifies every node
          confirmed / tested-negative / notable / flagged, so the
          result is auditable rather than just a picture
        - published as an HTML Artifact: inline SVG diagram + an
          evidence table tying each node back to its source run
   =============================================================================
```
