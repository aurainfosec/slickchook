#!/bin/sh
set -eu
: "${WPSCAN_API_TOKEN:?WPSCAN_API_TOKEN must be configured - see RUNNER.md's \"Configuring plugin API keys and secrets\"}"
exec wpscan --api-token "$WPSCAN_API_TOKEN" "$@"
