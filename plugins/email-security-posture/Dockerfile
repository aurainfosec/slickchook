FROM python:3.13-alpine
RUN pip install --no-cache-dir dnspython
COPY entrypoint.py /usr/local/bin/slickchook-email-security-posture-entrypoint.py
RUN chmod 0555 /usr/local/bin/slickchook-email-security-posture-entrypoint.py
ENTRYPOINT ["/usr/local/bin/slickchook-email-security-posture-entrypoint.py"]
