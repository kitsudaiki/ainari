FROM ubuntu:24.04 AS builder

ARG DEBIAN_FRONTEND=noninteractive

WORKDIR /app

RUN apt-get update && \
        apt-get install -y git \
                           ssh \
                           rustup \
                           libsqlite3-dev

COPY . .

RUN rustup install stable --no-self-update
RUN cargo build --release
RUN cp target/release/hanami /app/


FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install -y openssl libsqlite3-0 && \
    apt-get clean autoclean &&\
    apt-get autoremove --yes

# hanami
COPY --from=builder /app/hanami /usr/bin/hanami
CMD [ "hanami" ]
