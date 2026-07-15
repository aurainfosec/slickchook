FROM python:3.12-slim
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /opt/spiderfoot
RUN git clone --depth 1 https://github.com/smicallef/spiderfoot.git . \
    && pip install --no-cache-dir -r requirements.txt
ENTRYPOINT ["python", "sf.py"]
ENV HOME=/tmp
