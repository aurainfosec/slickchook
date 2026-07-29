#!/usr/bin/env bash
set -euo pipefail

: "${DATABASE_URL:?set DATABASE_URL to the target PostgreSQL database}"
ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
command -v psql >/dev/null || { echo "error: psql is required" >&2; exit 1; }
case "${DATABASE_URL}" in
  postgresql://*|postgres://*) ;;
  *) echo "error: DATABASE_URL must use postgresql:// or postgres://" >&2; exit 1 ;;
esac

shopt -s nullglob
migrations=("${ROOT}"/migrations/postgres/*.sql)
shopt -u nullglob
[[ ${#migrations[@]} -gt 0 ]] || { echo "error: no migration files found under migrations/postgres" >&2; exit 1; }

for migration in "${migrations[@]}"; do
  echo "Applying $(basename "${migration}")..."
  psql "${DATABASE_URL}" --set ON_ERROR_STOP=1 --file "${migration}"
done
echo "PostgreSQL schema applied successfully."
