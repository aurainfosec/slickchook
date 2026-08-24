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
# logic. Fixed by pointing HOME at a directory baked into the image
# (not /tmp, which is a tmpfs mount that starts empty every run) and
# triggering that one-time setup once, at build time, instead.
ENV HOME=/opt/jwt_tool_home
RUN python3 jwt_tool.py || true
RUN chmod -R 0777 /opt/jwt_tool_home
ENTRYPOINT ["python3", "/opt/jwt_tool/jwt_tool.py"]
