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
ENV HOME=/tmp
ENTRYPOINT ["python3", "/opt/jwt_tool/jwt_tool.py"]
