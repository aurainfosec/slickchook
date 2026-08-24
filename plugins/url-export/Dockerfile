# Self-contained stdlib-only Python tool - no dependency to install.
FROM python:3.13-alpine
COPY entrypoint.py /usr/local/bin/slickchook-url-export-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-url-export-entrypoint.py
ENTRYPOINT ["/usr/local/bin/slickchook-url-export-entrypoint.py"]
