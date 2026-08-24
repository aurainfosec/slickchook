# PyPI's own published theHarvester package (0.0.1) is drastically
# stale relative to the real current release (4.11.1) - confirmed
# live, not assumed - so this installs directly from the real, pinned
# release tag instead (theHarvester's own tags are unprefixed, e.g.
# "4.11.1", not "v4.11.1").
FROM python:3.13-slim
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN pip install --no-cache-dir "git+https://github.com/laramies/theHarvester.git@4.11.1"
COPY entrypoint.py /usr/local/bin/slickchook-theharvester-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-theharvester-entrypoint.py
ENTRYPOINT ["/usr/local/bin/slickchook-theharvester-entrypoint.py"]
