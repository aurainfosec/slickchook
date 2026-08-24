# sqlmap's own PyPI package is current and matches the real project
# (confirmed: same version 1.10.8, same maintainers) - a plain version
# pin is safe here, unlike several other tools this fleet has had to
# pin by commit due to a stale package index.
FROM python:3.13-slim
RUN pip install --no-cache-dir sqlmap==1.10.8
ENV HOME=/tmp
ENTRYPOINT ["sqlmap"]
