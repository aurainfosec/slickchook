FROM alpine:3.21 AS build

ARG TARGETARCH
RUN case "${TARGETARCH}" in \
        amd64) release_arch=amd64 ;; \
        arm64) release_arch=arm64 ;; \
        *) echo "unsupported architecture: ${TARGETARCH}" >&2; exit 1 ;; \
    esac \
    && apk add --no-cache ca-certificates wget unzip \
    && wget -qO /tmp/dnsx.zip \
       "https://github.com/projectdiscovery/dnsx/releases/download/v1.3.0/dnsx_1.3.0_linux_${release_arch}.zip" \
    && unzip /tmp/dnsx.zip dnsx -d /tmp \
    && chmod 0755 /tmp/dnsx

FROM alpine:3.21

RUN apk add --no-cache ca-certificates
COPY --from=build /tmp/dnsx /usr/local/bin/dnsx
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/dnsx"]
