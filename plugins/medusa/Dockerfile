FROM debian:bookworm-slim

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends \
        ca-certificates medusa \
    && rm -rf /var/lib/apt/lists/*

ENTRYPOINT ["/usr/bin/medusa"]
