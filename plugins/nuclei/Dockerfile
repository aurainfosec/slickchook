FROM alpine:3.21 AS templates
RUN apk add --no-cache git ca-certificates \
    && git clone --depth 1 https://github.com/projectdiscovery/nuclei-templates.git /opt/nuclei-templates

FROM projectdiscovery/nuclei:v3.11.0
COPY --from=templates /opt/nuclei-templates /opt/nuclei-templates
RUN test -n "$(find /opt/nuclei-templates -type f -name '*.yaml' -print -quit)"
