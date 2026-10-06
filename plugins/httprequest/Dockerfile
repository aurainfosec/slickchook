FROM alpine:3.21
RUN apk add --no-cache curl ca-certificates python3
COPY entrypoint.py /usr/local/bin/slickchook-httprequest-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-httprequest-entrypoint.py
ENTRYPOINT ["python3", "/usr/local/bin/slickchook-httprequest-entrypoint.py"]
