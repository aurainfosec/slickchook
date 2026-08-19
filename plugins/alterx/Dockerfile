FROM golang:1.24-alpine AS build
RUN apk add --no-cache git
# No official Docker image exists at this version: Docker Hub only ever
# published up to v0.0.6 (confirmed live - v0.1.0 is not there at all,
# and v0.0.6 lacks the entire pattern-mining feature set this manifest
# relies on). Building from source at the real, "v"-prefixed git tag
# instead.
RUN go install github.com/projectdiscovery/alterx/cmd/alterx@v0.1.0

FROM alpine:3.22
RUN apk add --no-cache ca-certificates
ENV HOME=/tmp
COPY --from=build /go/bin/alterx /usr/local/bin/alterx
ENTRYPOINT ["alterx"]
