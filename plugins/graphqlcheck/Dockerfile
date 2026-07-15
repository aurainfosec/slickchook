FROM python:3.13-alpine
COPY check.py /opt/check.py
RUN chmod 0555 /opt/check.py
ENTRYPOINT ["python", "/opt/check.py"]
