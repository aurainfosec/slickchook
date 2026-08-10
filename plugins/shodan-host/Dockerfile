FROM python:3.13-alpine
# The shodan package still imports pkg_resources at startup. That used to
# be part of setuptools, but setuptools removed pkg_resources entirely as
# of 81.0 (confirmed live: setuptools 84.0.0 installs with no pkg_resources
# module at all) - a bare `pip install setuptools` still leaves shodan
# broken with the exact same ModuleNotFoundError. Pin below that version.
RUN pip install --no-cache-dir shodan "setuptools<81"
COPY entrypoint.sh /usr/local/bin/slickchook-shodan-entrypoint.sh
RUN chmod 0555 /usr/local/bin/slickchook-shodan-entrypoint.sh
ENV HOME=/tmp
ENTRYPOINT ["/usr/local/bin/slickchook-shodan-entrypoint.sh"]
