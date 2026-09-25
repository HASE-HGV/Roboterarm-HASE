FROM archlinux:latest AS assembler

RUN pacman-key --init && \
    pacman-key --populate archlinux && \
    pacman -Syu --noconfirm && \
    pacman -S --noconfirm arch-install-scripts rust sudo

RUN bash -c 'echo "%wheel ALL=(ALL:ALL) ALL" | sudo tee -a /etc/sudoers'

RUN mkdir /src && mkdir /export

RUN useradd -m mainuser
RUN usermod -aG wheel mainuser

COPY ./docker/include /include
COPY ./rustctl /src/rustctl

RUN cd /src/rustctl
RUN cargo build --release --bin export-rustctl
RUN cp /src/rustctl/target/release/export-rustctl /export/rustctl

ENTRYPOINT ["/include/entrypoint.root.sh"]
