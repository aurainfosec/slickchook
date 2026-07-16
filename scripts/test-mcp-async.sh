#!/usr/bin/env bash
set -euo pipefail

MCP_URL="${MCP_URL:-http://127.0.0.1:8080/sse}"
REQUEST="${1:?usage: $0 '{\"plugin\":...,\"input\":{...}}'}"
WORK_DIR=$(mktemp -d)
trap 'kill "${SSE_PID:-}" 2>/dev/null || true; rm -rf "$WORK_DIR"' EXIT
STREAM="$WORK_DIR/stream"
curl --silent --no-buffer "$MCP_URL" >"$STREAM" & SSE_PID=$!
for _ in $(seq 1 50); do
  ENDPOINT=$(sed -n 's/^data: //p' "$STREAM" | grep '^/message?' | head -n1 || true)
  [[ -n "$ENDPOINT" ]] && break
  sleep .1
done
[[ -n "${ENDPOINT:-}" ]] || { echo 'MCP SSE endpoint was not received' >&2; exit 1; }
POST_URL="${MCP_URL%/sse}${ENDPOINT}"
post() { curl --silent --fail -H 'content-type: application/json' --data "$1" "$POST_URL" >/dev/null; }
post '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"slickchook-async-test","version":"1"}}}'
post '{"jsonrpc":"2.0","method":"notifications/initialized"}'
post "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"start_plugin\",\"arguments\":$REQUEST}}"
for _ in $(seq 1 30); do
  TEXT=$(sed -n 's/^data: //p' "$STREAM" | grep '"id":2' | tail -n1 | jq -r '.result.content[0].text // empty' 2>/dev/null || true)
  JOB_ID=$(jq -r '.job_id // empty' <<<"$TEXT" 2>/dev/null || true)
  [[ -n "$JOB_ID" ]] && break
  sleep 1
done
[[ -n "${JOB_ID:-}" ]] || { echo 'failed to obtain job id' >&2; exit 1; }
echo "MCP async job started"
echo "  job_id: $JOB_ID"
echo "  status: running"
for _ in $(seq 1 900); do
  post "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"plugin_status\",\"arguments\":{\"job_id\":\"$JOB_ID\"}}}"
  TEXT=$(sed -n 's/^data: //p' "$STREAM" | grep '"id":3' | tail -n1 | jq -r '.result.content[0].text // empty' 2>/dev/null || true)
  if [[ -n "$TEXT" ]] && jq -e '.status == "success" or .status == "failed" or .status == "cancelled"' >/dev/null 2>&1 <<<"$TEXT"; then
    echo
    echo "MCP async job completed"
    plugin=$(jq -r '.plugin // "unknown"' <<<"$TEXT")
    status=$(jq -r '.status // "unknown"' <<<"$TEXT")
    exit_code=$(jq -r '.exit_code // "none"' <<<"$TEXT")
    artifact_dir=$(jq -r '.artifact_dir // "none"' <<<"$TEXT")
    echo "  plugin: $plugin"
    echo "  status: $status"
    echo "  exit code: $exit_code"
    echo "  artifact directory: $artifact_dir"
    echo
    echo "Detailed result:"
    jq . <<<"$TEXT"
    for key in stdout_artifact stderr_artifact; do
      path=$(jq -r ".${key} // empty" <<<"$TEXT")
      [[ -f "$path" ]] && { echo "--- $key snippet ---"; sed -n '1,80p' "$path"; }
    done
    exit 0
  fi
  sleep 1
done
echo 'timed out waiting for async plugin' >&2
exit 1
