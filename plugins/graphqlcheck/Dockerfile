FROM python:3.13-alpine
COPY check.py /opt/check.py
COPY fields.txt /fields.txt
RUN chmod 0555 /opt/check.py && chmod 0644 /fields.txt && test -s /fields.txt
ENTRYPOINT ["python", "/opt/check.py"]
