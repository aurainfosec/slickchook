#!/bin/sh
set -eu
: "${VT_API_KEY:?VT_API_KEY must be configured on the slickchook service - see RUNNER.md's \"Configuring plugin API keys and secrets\"}"
exec vt --apikey "$VT_API_KEY" "$@"
