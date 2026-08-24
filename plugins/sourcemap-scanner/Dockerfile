# Self-contained stdlib-only Python tool - no third-party dependency to
# install beyond the base image.
FROM python:3.13-alpine
COPY entrypoint.py /usr/local/bin/slickchook-sourcemap-scanner-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-sourcemap-scanner-entrypoint.py
ENTRYPOINT ["/usr/local/bin/slickchook-sourcemap-scanner-entrypoint.py"]
