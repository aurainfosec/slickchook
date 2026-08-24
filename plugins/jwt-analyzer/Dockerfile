# No PyPI package exists for jwt_tool at all (checked both jwt-tool and
# jwt_tool package names - neither exists) - built from source, pinned
# to its real latest release tag (v2.3.0, 2025-05-01; not actively
# maintained since, but still the standard tool for this with no
# better-maintained alternative found).
FROM python:3.13-slim
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 --branch v2.3.0 https://github.com/ticarpi/jwt_tool.git /opt/jwt_tool
WORKDIR /opt/jwt_tool
RUN pip install --no-cache-dir -r requirements.txt
# jwt_tool writes a one-time config + demo RSA/EC keypairs to
# ~/.jwt_tool on its first run, then unconditionally exits - it expects
# a *second* invocation to actually do anything. Every plugin run here
# is a fresh, ephemeral container, so that first-run behavior would
# fire on every single call and it would never reach real decode
# logic. Fixed by baking that one-time setup into the image at build
# time instead, into a directory the runtime entrypoint script copies
# from into the writable tmpfs on every run (this fleet's containers
# always run with a read-only root filesystem at runtime - confirmed
# live that jwt_tool still needs to write a log line and a marker file
# under ~/.jwt_tool on every single invocation, not just the first, so
# the baked-in copy alone isn't enough - see entrypoint.sh).
ENV HOME=/opt/jwt_tool_home
RUN python3 jwt_tool.py || true
RUN chmod -R a+rX /opt/jwt_tool_home
COPY entrypoint.sh /usr/local/bin/slickchook-jwt-analyzer-entrypoint.sh
RUN chmod 0555 /usr/local/bin/slickchook-jwt-analyzer-entrypoint.sh
ENTRYPOINT ["/usr/local/bin/slickchook-jwt-analyzer-entrypoint.sh"]
