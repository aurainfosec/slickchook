# sippts's own pyproject.toml declares its dependencies dynamically from a
# requirements.txt file - confirmed live (not assumed from pyproject.toml
# alone) that this file doesn't actually exist anywhere in this tag's real
# repo contents, a real upstream packaging bug. Writing it here ourselves,
# matching setup.py's own real install_requires list exactly, so `pip
# install .` can actually resolve its metadata instead of failing outright.
FROM python:3.13-slim

# netifaces (a dependency pulled in transitively for sipscan.py's own use)
# is a C extension with no prebuilt wheel for this Python/platform
# combination - confirmed live, a real build failure, not assumed -
# needing gcc/libc headers to build from source, hence build-essential.
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates build-essential \
    && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 --branch v4.1.2 https://github.com/Pepelux/sippts.git /opt/sippts
WORKDIR /opt/sippts
RUN printf 'netifaces\nrequests\nIPy\nscapy\npyshark\nwebsocket-client\nrel\n' > requirements.txt \
    && pip install --no-cache-dir .

ENTRYPOINT ["sippts"]
