FROM golang:1.24-alpine AS build
RUN apk add --no-cache git
RUN go install -v github.com/melvinsh/subfaster/v2/cmd/subfaster@v2.18.0

FROM alpine:3.22
RUN apk add --no-cache ca-certificates
ENV HOME=/tmp
COPY --from=build /go/bin/subfaster /usr/local/bin/subfaster
ENTRYPOINT ["/usr/local/bin/subfaster"]
