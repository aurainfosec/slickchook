# No pip package exists for pagodo (it's a pair of scripts, not a
# library) - built from a real, pinned git tag instead.
FROM python:3.13-slim-bookworm
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 --branch v2.7.0 https://github.com/opsdisk/pagodo.git /opt/pagodo
WORKDIR /opt/pagodo
RUN pip install --no-cache-dir -r requirements.txt
# Bake the GHDB dork corpus into the image at build time rather than
# scraping it live on every run - ties dork-list freshness to a plugin
# rebuild/reinstall, and keeps every real run's own network activity
# limited to the actual Google queries themselves.
RUN python ghdb_scraper.py -s -i
COPY entrypoint.py /usr/local/bin/slickchook-pagodo-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-pagodo-entrypoint.py
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/slickchook-pagodo-entrypoint.py"]
