#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?set DATABASE_URL to the target PostgreSQL database}"
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v psql >/dev/null || { echo "error: psql is required" >&2; exit 1; }
case "${DATABASE_URL}" in
  postgresql://*|postgres://*) ;;
  *) echo "error: DATABASE_URL must use postgresql:// or postgres://" >&2; exit 1 ;;
esac

psql "${DATABASE_URL}" --set ON_ERROR_STOP=1 \
  --file "${ROOT}/migrations/postgres/001_initial.sql"
echo "PostgreSQL schema applied successfully."
