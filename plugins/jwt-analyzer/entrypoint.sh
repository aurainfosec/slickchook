#!/bin/sh
set -eu
# The pre-built jwt_tool config/keys baked into the image at
# /opt/jwt_tool_home are on a read-only filesystem at runtime (this
# fleet's containers always run with --read-only) - jwt_tool still
# writes a log line and a marker file under ~/.jwt_tool on every
# invocation regardless, so copy the read-only template into the
# writable tmpfs /tmp on each run and point HOME there.
cp -r /opt/jwt_tool_home/.jwt_tool /tmp/.jwt_tool
export HOME=/tmp
exec python3 /opt/jwt_tool/jwt_tool.py "$@"
