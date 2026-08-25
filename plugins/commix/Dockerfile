# commix's own PyPI package is drastically stale (0.1) - confirmed
# live, not assumed - built from source instead, pinned to the real
# current release tag. setup.py declares a console-script entry point
# (commix = src.core.main:entry_point), but that entry point doesn't
# actually exist in src/core/main.py at this tag - confirmed live, the
# installed console script fails with a real ImportError. commix.py at
# the repo root is the tool's own real, intended entry point (it
# checks `if __name__ == '__main__'` and imports `src.core.main` as a
# plain module-level import, not through a packaged entry point) - run
# that directly instead, matching how the tool is actually designed to
# be invoked. No pip install needed at all: no requirements.txt exists
# in the repo and setup.py declares no install_requires - it's a
# self-contained script needing only its own bundled src/ and
# thirdparty/ code plus the standard library.
FROM python:3.13-slim
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 --branch v4.1 https://github.com/commixproject/commix.git /opt/commix
WORKDIR /opt/commix
ENTRYPOINT ["python3", "/opt/commix/commix.py"]
