# Same Debian package as the ike-scan plugin - psk-crack is bundled in
# the same real ike-scan package (1.9.5-1+b1 in bookworm, confirmed
# matching upstream's real latest tag). python3 is added only to run this
# plugin's own thin argument-rewriting wrapper - see entrypoint.py.
FROM debian:bookworm-slim

RUN apt-get update \
    && DEBIAN_FRONTEND=noninteractive apt-get install --yes --no-install-recommends \
        ca-certificates ike-scan python3 \
    && rm -rf /var/lib/apt/lists/*

COPY entrypoint.py /usr/local/bin/slickchook-psk-crack-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-psk-crack-entrypoint.py
ENTRYPOINT ["/usr/local/bin/slickchook-psk-crack-entrypoint.py"]
