FROM wpscanteam/wpscan:latest
# wpscanteam/wpscan's own Dockerfile sets a non-root USER before its final
# layer, which carries over here. COPY always writes as root:root regardless
# of the current effective user, so without switching back to root first,
# the inherited non-root user can't chmod a file it doesn't own.
USER root
COPY entrypoint.sh /usr/local/bin/slickchook-wpscan-entrypoint.sh
RUN chmod 0555 /usr/local/bin/slickchook-wpscan-entrypoint.sh
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/slickchook-wpscan-entrypoint.sh"]
