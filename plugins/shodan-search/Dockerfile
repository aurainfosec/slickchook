FROM python:3.13-alpine
# The shodan package still imports pkg_resources (from setuptools) at
# startup, but recent python:*-alpine images no longer bundle setuptools by
# default - without it, `shodan` fails immediately with
# ModuleNotFoundError: No module named 'pkg_resources'.
RUN pip install --no-cache-dir shodan setuptools
COPY entrypoint.sh /usr/local/bin/slickchook-shodan-entrypoint.sh
RUN chmod 0555 /usr/local/bin/slickchook-shodan-entrypoint.sh
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/slickchook-shodan-entrypoint.sh"]
