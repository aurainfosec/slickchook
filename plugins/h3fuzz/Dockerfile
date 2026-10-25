FROM alpine:3.21 AS build

RUN case "$(apk --print-arch)" in \
        x86_64)  release_arch=amd64 ;; \
        aarch64) release_arch=arm64 ;; \
        *) echo "unsupported architecture: $(apk --print-arch)" >&2; exit 1 ;; \
    esac \
    && apk add --no-cache ca-certificates wget tar \
    && wget -qO /tmp/h3fuzz.tar.gz \
       "https://github.com/assetnote/h3fuzz/releases/download/v0.1.0/h3fuzz_0.1.0_linux_${release_arch}.tar.gz" \
    && tar -xzf /tmp/h3fuzz.tar.gz -C /tmp h3fuzz \
    && chmod 0755 /tmp/h3fuzz

FROM alpine:3.21

RUN apk add --no-cache ca-certificates
COPY --from=build /tmp/h3fuzz /usr/local/bin/h3fuzz
COPY wordlist.txt /wordlist.txt
COPY wordlist-cms.txt /wordlist-cms.txt
COPY wordlist-api.txt /wordlist-api.txt
RUN chmod 0644 /wordlist.txt /wordlist-cms.txt /wordlist-api.txt \
    && test -s /wordlist.txt && test -s /wordlist-cms.txt && test -s /wordlist-api.txt
ENTRYPOINT ["/usr/local/bin/h3fuzz"]
