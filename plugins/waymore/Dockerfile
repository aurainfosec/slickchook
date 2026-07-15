FROM python:3.12-slim-bookworm

RUN python -m pip install --no-cache-dir waymore==8.9

ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/waymore"]
