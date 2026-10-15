FROM alpine:3.21 AS build

RUN case "$(apk --print-arch)" in \
        x86_64)  release_arch=x86_64 ;; \
        aarch64) release_arch=aarch64 ;; \
        *) echo "unsupported architecture: $(apk --print-arch)" >&2; exit 1 ;; \
    esac \
    && apk add --no-cache ca-certificates wget unzip \
    && wget -qO /tmp/feroxbuster.zip \
       "https://github.com/epi052/feroxbuster/releases/download/v2.13.1/${release_arch}-linux-feroxbuster.zip" \
    && unzip /tmp/feroxbuster.zip feroxbuster -d /tmp \
    && chmod 0755 /tmp/feroxbuster

FROM alpine:3.21

RUN apk add --no-cache ca-certificates
COPY --from=build /tmp/feroxbuster /usr/local/bin/feroxbuster
COPY wordlist.txt /wordlist.txt
COPY wordlist-cms.txt /wordlist-cms.txt
COPY wordlist-api.txt /wordlist-api.txt
RUN chmod 0644 /wordlist.txt /wordlist-cms.txt /wordlist-api.txt \
    && test -s /wordlist.txt && test -s /wordlist-cms.txt && test -s /wordlist-api.txt
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/feroxbuster"]
