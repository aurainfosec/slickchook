#!/usr/bin/env bash
set -euo pipefail

readonly MCP_URL="${MCP_URL:-http://127.0.0.1:8080/sse}"
readonly RESPONSE_WAIT_TICKS="${RESPONSE_WAIT_TICKS:-300}"
HUMAN=false

if [[ ${1:-} == "--human" ]]; then
  HUMAN=true
  shift
fi
WORK_DIR=$(mktemp -d)
STREAM="${WORK_DIR}/stream"
SSE_PID=""

cleanup() {
  if [[ -n ${SSE_PID} ]]; then
    kill "${SSE_PID}" 2>/dev/null || true
    wait "${SSE_PID}" 2>/dev/null || true
  fi
  rm -rf -- "${WORK_DIR}"
}
trap cleanup EXIT

curl --silent --show-error --no-buffer "${MCP_URL}" >"${STREAM}" &
SSE_PID=$!

for _ in $(seq 1 50); do
  ENDPOINT=$(sed -n 's/^data: //p' "${STREAM}" | grep '^/message?' | head -n 1 || true)
  [[ -n ${ENDPOINT} ]] && break
  sleep 0.1
done
[[ -n ${ENDPOINT:-} ]] || { echo "MCP SSE endpoint was not received" >&2; exit 1; }

BASE=${MCP_URL%/sse}
POST_URL="${BASE}${ENDPOINT}"

post() {
  curl --silent --show-error --fail \
    -H 'content-type: application/json' \
    --data "$1" "${POST_URL}" >/dev/null
}

wait_for_id() {
  local id=$1
  for _ in $(seq 1 "${RESPONSE_WAIT_TICKS}"); do
    if grep -q "\"id\":${id}[,}]" "${STREAM}"; then
      return 0
    fi
    sleep 0.1
  done
  echo "timed out waiting for MCP response id ${id}" >&2
  return 1
}

post '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","capabilities":{},"clientInfo":{"name":"slickchook-live-test","version":"1"}}}'
wait_for_id 1
post '{"jsonrpc":"2.0","method":"notifications/initialized"}'
post '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
wait_for_id 2
post '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_plugins","arguments":{}}}'
wait_for_id 3

id=10
for call in "$@"; do
  post "{\"jsonrpc\":\"2.0\",\"id\":${id},\"method\":\"tools/call\",\"params\":{\"name\":\"run_plugin\",\"arguments\":${call}}}"
  wait_for_id "${id}"
  id=$((id + 1))
done

if [[ ${HUMAN} == true ]]; then
  command -v jq >/dev/null || {
    echo "--human requires jq (install it with: sudo apt-get install jq)" >&2
    exit 1
  }
  while IFS= read -r line; do
    [[ ${line} == \{* ]] || continue
    printf '\n--- MCP response ---\n'
    jq . <<<"${line}"
  done < <(sed -n 's/^data: //p' "${STREAM}")
else
  sed -n 's/^data: //p' "${STREAM}"
fi
