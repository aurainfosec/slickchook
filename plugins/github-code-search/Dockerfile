# Official gh CLI release tarball, pinned to an exact version - no
# official Docker image is needed for a single static-ish Go binary, and
# this avoids alpine/musl vs glibc mismatches some Go release binaries hit.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://github.com/cli/cli/releases/download/v2.97.0/gh_2.97.0_linux_amd64.tar.gz \
    | tar -xz -C /usr/local --strip-components=1 gh_2.97.0_linux_amd64/bin/gh
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/gh"]
