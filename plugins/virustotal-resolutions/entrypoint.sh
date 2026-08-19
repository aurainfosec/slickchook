#!/bin/sh
set -eu
: "${VT_API_KEY:?VT_API_KEY must be configured on the slickchook service - see RUNNER.md's \"Configuring plugin API keys and secrets\"}"
# vt-cli only registers per-relationship subcommands (e.g. "resolutions")
# from a local cache file that `vt init` writes - without it, `domain
# resolutions` doesn't exist as a command at all (confirmed live: it
# silently falls through to the base `domain` command instead, sending
# "resolutions" itself as a domain name to VirusTotal's API). Since
# --apikey is already set, `vt init` skips its interactive prompt
# entirely (confirmed in vt-cli's own source, cmd/init.go) and just
# writes the cache non-interactively.
vt --apikey "$VT_API_KEY" init >/dev/null
exec vt --apikey "$VT_API_KEY" "$@"
