# No git tags exist for this repo at all (confirmed live) - pinning by
# commit SHA instead, the only reproducible option available.
FROM python:3-alpine
RUN apk add --no-cache git
RUN git clone https://github.com/Mebus/cupp.git /opt/cupp \
    && cd /opt/cupp \
    && git checkout b77f1ae66c10ab2d0fa09c67017949fb5ba187aa
COPY entrypoint.py /usr/local/bin/slickchook-cupp-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-cupp-entrypoint.py
ENTRYPOINT ["/usr/local/bin/slickchook-cupp-entrypoint.py"]
