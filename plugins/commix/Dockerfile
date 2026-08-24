# commix's own PyPI package is drastically stale (0.1) - confirmed
# live, not assumed - built from source instead, pinned to the real
# current release tag. Its own setup.py defines a real console-script
# entry point (commix = src.core.main:entry_point), so a plain `pip
# install .` from the pinned checkout works correctly.
FROM python:3.13-slim
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 --branch v4.1 https://github.com/commixproject/commix.git /opt/commix
RUN pip install --no-cache-dir /opt/commix
ENTRYPOINT ["commix"]
