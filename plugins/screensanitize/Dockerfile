FROM alpine:3.22
RUN apk add --no-cache python3 py3-pillow tesseract-ocr tesseract-ocr-data-eng
COPY sanitize.py /opt/sanitize.py
RUN chmod 0555 /opt/sanitize.py
ENTRYPOINT ["python3", "/opt/sanitize.py"]
