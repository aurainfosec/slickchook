FROM golang:1.24-alpine AS build
RUN apk add --no-cache git ca-certificates
WORKDIR /src
RUN git clone --depth 1 https://github.com/jaeles-project/gospider.git . \
    && go build -trimpath -ldflags='-s -w' -o /usr/local/bin/gospider ./

FROM alpine:3.21
RUN apk add --no-cache ca-certificates
COPY --from=build /usr/local/bin/gospider /usr/local/bin/gospider
ENTRYPOINT ["/usr/local/bin/gospider"]
