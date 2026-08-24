# wafw00f's PyPI package matches its latest GitHub release exactly
# (both 2.4.2, confirmed live) - unlike some other tools this fleet has
# had to pin by commit instead, a plain version pin is safe here.
FROM python:3.13-slim
RUN pip install --no-cache-dir wafw00f==2.4.2
ENTRYPOINT ["wafw00f"]
