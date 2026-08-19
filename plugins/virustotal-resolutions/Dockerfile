FROM golang:1.24-alpine AS build
RUN apk add --no-cache git
# vt-cli's git tags are "1.3.1" etc., without the "v" prefix Go's module
# system requires to resolve a tag as a version at all (confirmed live:
# `go install .../vt@v1.3.1` fails with "unknown revision vt/v1.3.1").
# Pinned to the exact commit the "1.3.1" tag points to instead - equally
# reproducible, sidesteps the tag-naming mismatch entirely.
RUN go install github.com/VirusTotal/vt-cli/vt@c7e4cdbde9778757444d96e6321785fa6972beed

FROM alpine:3.22
RUN apk add --no-cache ca-certificates
ENV HOME=/tmp
COPY --from=build /go/bin/vt /usr/local/bin/vt
COPY entrypoint.sh /usr/local/bin/slickchook-vt-entrypoint.sh
RUN chmod 0555 /usr/local/bin/slickchook-vt-entrypoint.sh
ENTRYPOINT ["/usr/local/bin/slickchook-vt-entrypoint.sh"]
