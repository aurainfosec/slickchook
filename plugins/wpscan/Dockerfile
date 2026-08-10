FROM wpscanteam/wpscan:latest
COPY entrypoint.sh /usr/local/bin/slickchook-wpscan-entrypoint.sh
RUN chmod 0555 /usr/local/bin/slickchook-wpscan-entrypoint.sh
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/slickchook-wpscan-entrypoint.sh"]
