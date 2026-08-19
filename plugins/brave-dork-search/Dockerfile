# No official CLI/SDK exists for the Brave Search API - entrypoint.py is a
# from-scratch stdlib-only HTTP client, so no dependency install is needed
# beyond the base Python image.
FROM python:3.13-alpine
COPY entrypoint.py /usr/local/bin/slickchook-brave-dork-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-brave-dork-entrypoint.py
ENTRYPOINT ["/usr/local/bin/slickchook-brave-dork-entrypoint.py"]
