FROM ubuntu:24.04 AS builder

ARG DEBIAN_FRONTEND=noninteractive

WORKDIR /app

RUN apt-get update && \
        apt-get install -y git \
                           ssh \
                           gcc \
                           pkg-config \
                           libssl-dev \
                           rustup \
                           libsqlite3-dev

COPY . .

RUN rustup install stable --no-self-update
RUN cargo build --release
RUN cp target/release/sakura /app/
RUN cp target/release/miko /app/
RUN cp target/release/bento /app/

# ---------------------------------------------------

FROM ubuntu:24.04 AS sakura

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install -y openssl libsqlite3-0 && \
    apt-get clean autoclean &&\
    apt-get autoremove --yes

# sakura
COPY --from=builder /app/sakura /usr/bin/sakura
CMD [ "sakura" ]

# ---------------------------------------------------

FROM ubuntu:24.04 AS miko

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install -y openssl libsqlite3-0 && \
    apt-get clean autoclean &&\
    apt-get autoremove --yes

# miko
COPY --from=builder /app/miko /usr/bin/miko
CMD [ "miko" ]

# ---------------------------------------------------

FROM ubuntu:24.04 AS bento

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install -y openssl libsqlite3-0 && \
    apt-get clean autoclean &&\
    apt-get autoremove --yes

# bento
COPY --from=builder /app/bento /usr/bin/bento
CMD [ "bento" ]
