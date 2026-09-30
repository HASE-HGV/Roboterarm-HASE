ARG BUILDPLATFORM=linux/amd64
FROM --platform=$BUILDPLATFORM archlinux:latest AS assembler

RUN sed -i 's/#DisableSandbox/DisableSandbox/' /etc/pacman.conf && \
    pacman-key --init && \
    pacman-key --populate archlinux && \
    pacman --noconfirm -Syu && \
    pacman -S --noconfirm arch-install-scripts rustup sudo gcc aarch64-linux-gnu-gcc binutils

RUN bash -c 'echo "%wheel ALL=(ALL:ALL) ALL" | sudo tee -a /etc/sudoers'
RUN mkdir -p /src /export
RUN useradd -m mainuser && usermod -aG wheel mainuser

COPY ./docker/include /include
COPY ./rustctl /src/rustctl

RUN rustup default stable && \
    rustup target add aarch64-unknown-linux-gnu

WORKDIR /src/rustctl
ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc
RUN cargo build --release --target aarch64-unknown-linux-gnu && \
    cp target/aarch64-unknown-linux-gnu/release/rustctl /export/export_build

FROM scratch AS export
COPY --from=assembler /export/export_build /export_build
