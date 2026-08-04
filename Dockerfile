FROM rust:bookworm

ENV DEBIAN_FRONTEND=noninteractive \
    XDG_CACHE_HOME=/home/stealth/.cache

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        chromium \
        chromium-sandbox \
        fontconfig \
        fonts-crosextra-caladea \
        fonts-crosextra-carlito \
        fonts-liberation \
        fonts-noto-color-emoji \
        libgbm1 \
        libssl-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash stealth \
    && mkdir -p /workspace /usr/local/share/fonts/windows /home/stealth/.cache/fontconfig

COPY container/fonts/ /usr/local/share/fonts/windows/
COPY container/entrypoint.sh /usr/local/bin/stealth-oxide-entrypoint
RUN chmod 0755 /usr/local/bin/stealth-oxide-entrypoint \
    && fc-cache -f \
    && chown -R stealth:stealth /workspace /home/stealth

WORKDIR /workspace
COPY --chown=stealth:stealth Cargo.toml Cargo.lock ./
COPY --chown=stealth:stealth src ./src
COPY --chown=stealth:stealth tests ./tests
COPY --chown=stealth:stealth examples ./examples

USER stealth
RUN cargo build --tests --examples

ENTRYPOINT ["/usr/local/bin/stealth-oxide-entrypoint"]
CMD ["cargo", "test", "--test", "device_environment", "--", "--ignored", "--nocapture"]
