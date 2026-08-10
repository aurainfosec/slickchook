#!/bin/sh
set -eu
: "${SHODAN_API_KEY:?SHODAN_API_KEY must be configured - see RUNNER.md's \"Configuring plugin API keys and secrets\"}"
shodan init "$SHODAN_API_KEY" >/dev/null
exec shodan "$@"
