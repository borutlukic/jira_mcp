FROM mcr.microsoft.com/devcontainers/rust:trixie

# Windows cross-compilation
RUN apt-get update && apt-get install -y mingw-w64 && rm -rf /var/lib/apt/lists/*

# osxcross build dependencies
RUN apt-get update && apt-get install -y \
    clang cmake patch python3 \
    libssl-dev liblzma-dev libxml2-dev \
    xz-utils bzip2 cpio libbz2-dev zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

# Clone MacOSX SDKs, package MacOSX11.3.sdk for osxcross, build osxcross
RUN git clone --depth=1 https://github.com/phracker/MacOSX-SDKs /opt/macos-sdk \
    && git clone https://github.com/tpoechtrager/osxcross /tmp/osxcross-build \
    && tar -C /opt/macos-sdk -cJf /tmp/osxcross-build/tarballs/MacOSX11.3.sdk.tar.xz MacOSX11.3.sdk \
    && cd /tmp/osxcross-build \
    && UNATTENDED=1 OSX_VERSION_MIN=10.13 TARGET_DIR=/opt/osxcross ./build.sh \
    && rm -rf /tmp/osxcross-build

ENV PATH="/opt/osxcross/bin:${PATH}"
ENV LD_LIBRARY_PATH="/opt/osxcross/lib"