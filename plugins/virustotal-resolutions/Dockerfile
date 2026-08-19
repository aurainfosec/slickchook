FROM golang:1.24-alpine AS build
RUN apk add --no-cache git
RUN go install github.com/VirusTotal/vt-cli/vt@v1.3.1

FROM alpine:3.22
RUN apk add --no-cache ca-certificates
ENV HOME=/tmp
COPY --from=build /go/bin/vt /usr/local/bin/vt
COPY entrypoint.sh /usr/local/bin/slickchook-vt-entrypoint.sh
RUN chmod 0555 /usr/local/bin/slickchook-vt-entrypoint.sh
ENTRYPOINT ["/usr/local/bin/slickchook-vt-entrypoint.sh"]
