FROM golang:1.24-alpine AS build
RUN apk add --no-cache git
RUN go install github.com/assetnote/kiterunner/cmd/kiterunner@v1.0.2

FROM alpine:3.22
RUN apk add --no-cache ca-certificates curl
COPY --from=build /go/bin/kiterunner /usr/local/bin/kr
RUN mkdir -p /opt/kiterunner \
    && curl -fsSL https://wordlists-cdn.assetnote.io/data/kiterunner/routes-small.kite.tar.gz \
       | tar -xz -C /opt/kiterunner \
    && test -s /opt/kiterunner/routes-small.kite
ENTRYPOINT ["kr"]
