FROM python:3.12-alpine
RUN apk add --no-cache ca-certificates
COPY collect_ct.py /usr/local/bin/collect_ct.py
RUN chmod 0755 /usr/local/bin/collect_ct.py
ENTRYPOINT ["python", "/usr/local/bin/collect_ct.py"]
