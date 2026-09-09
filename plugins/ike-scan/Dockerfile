# Debian's ike-scan package (1.9.5-1+b1 in bookworm) matches the real
# upstream project's latest tag exactly - confirmed via royhills/ike-scan's
# own git tags and ChangeLog, not assumed. No official Docker image exists
# worth wrapping instead.
FROM debian:bookworm-slim

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends \
        ca-certificates ike-scan \
    && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/usr/bin/ike-scan"]
