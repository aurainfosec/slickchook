FROM python:3.13-alpine
RUN pip install --no-cache-dir shodan
COPY entrypoint.sh /usr/local/bin/slickchook-shodan-entrypoint.sh
RUN chmod 0555 /usr/local/bin/slickchook-shodan-entrypoint.sh
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/slickchook-shodan-entrypoint.sh"]
