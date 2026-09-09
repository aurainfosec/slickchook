#!/bin/sh
set -eu
cp -r /opt/trivy-db-template /tmp/trivy-db
exec trivy --cache-dir /tmp/trivy-db "$@"
