FROM rust:bookworm

ENV DEBIAN_FRONTEND=noninteractive

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
        libegl1-mesa \
        libgl1-mesa-dri \
        libgbm1 \
        libssl-dev \
        mesa-utils \
        mesa-vulkan-drivers \
        openbox \
        pkg-config \
        espeak-ng \
        speech-dispatcher \
        speech-dispatcher-espeak-ng \
        tint2 \
        x11-utils \
        xvfb \
    && rm -rf /var/lib/apt/lists/*

ENV DISPLAY=:99 \
    GALLIUM_DRIVER=llvmpipe \
    LIBGL_ALWAYS_SOFTWARE=1 \
    STEALTH_OXIDE_HEADFUL=1 \
    STEALTH_OXIDE_USE_NATIVE_SCREEN=1 \
    STEALTH_OXIDE_USE_MESA=1 \
    XDG_CACHE_HOME=/home/stealth/.cache

RUN useradd --create-home --shell /bin/bash stealth \
    && mkdir -p \
        /etc/stealth-oxide \
        /home/stealth/.cache/fontconfig \
        /home/stealth/.cache/fontconfig-windows \
        /usr/local/share/fonts/windows \
        /workspace

COPY container/fonts/ /usr/local/share/fonts/windows/
COPY container/entrypoint.sh /usr/local/bin/stealth-oxide-entrypoint
COPY container/tint2rc /etc/stealth-oxide/tint2rc
COPY container/windows-fonts.conf /etc/stealth-oxide/windows-fonts.conf
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
