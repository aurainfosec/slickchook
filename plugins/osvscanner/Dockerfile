FROM golang:1.24-alpine AS build
RUN apk add --no-cache git
RUN go install github.com/google/osv-scanner/v2/cmd/osv-scanner@v2.0.2 \
    && mkdir -p /out \
    && cp /go/bin/osv-scanner /out/osv-scanner

FROM alpine:3.22
COPY --from=build /out/osv-scanner /usr/local/bin/osv-scanner
RUN mkdir -p /opt/osv-db
# Refresh this database during image builds; runtime scanning remains offline.
ENV OSV_SCANNER_LOCAL_DB_CACHE_DIRECTORY=/opt/osv-db
RUN mkdir -p /tmp/osv-seed \
    && printf '{"name":"offline-seed","version":"0.0.0","lockfileVersion":3,"requires":true,"packages":{"":{"name":"offline-seed","version":"0.0.0","dependencies":{"lodash":"4.17.21"}},"node_modules/lodash":{"version":"4.17.21","resolved":"https://registry.npmjs.org/lodash/-/lodash-4.17.21.tgz","integrity":"sha512-"}}}\n' > /tmp/osv-seed/package-lock.json \
    && (/usr/local/bin/osv-scanner scan source --offline-vulnerabilities --download-offline-databases -L /tmp/osv-seed/package-lock.json || test $? -eq 1)
ENTRYPOINT ["osv-scanner"]
