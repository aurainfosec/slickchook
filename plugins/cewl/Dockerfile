# No fixed-version published image exists to reference directly (the
# upstream repo publishes to GHCR under a moving tag scheme, and Docker
# Hub has no digininja/cewl repository at all) - building from source,
# pinned to the exact 6.2.1 release tag, closely mirroring upstream's
# own reference Dockerfile.
FROM ruby:3-alpine
LABEL org.opencontainers.image.description="CeWL, built from source pinned to the 6.2.1 tag"
ENV RUBYOPT="rrubygems"
ENV HOME=/tmp
RUN apk add --no-cache git gcompat
RUN set -ex \
    && apk add --no-cache --virtual .build-deps build-base \
    && git clone --depth 1 --branch 6.2.1 https://github.com/digininja/CeWL.git /usr/src/CeWL \
    && cd /usr/src/CeWL \
    && gem install bundler \
    && bundle install \
    && apk del .build-deps
WORKDIR /usr/src/CeWL
ENTRYPOINT ["/usr/src/CeWL/cewl.rb"]
