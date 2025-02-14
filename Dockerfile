FROM ubuntu:24.04 AS builder

ARG DEBIAN_FRONTEND=noninteractive

WORKDIR /app

RUN apt-get update && \
        apt-get install -y clang-15 \
                           make \
                           cmake \
                           bison \
                           flex \
                           git \
                           ssh \
                           libssl-dev \
                           libcrypto++-dev \
                           libboost-dev \
                           nlohmann-json3-dev \
                           uuid-dev  \
                           libsqlite3-dev \
                           protobuf-compiler && \
        ln -s /usr/bin/clang++-15 /usr/bin/clang++ && \
        ln -s /usr/bin/clang-15 /usr/bin/clang

COPY . .

RUN rm  -f src/libraries/hanami_messages/protobuffers/hanami_messages.proto3.pb.cc src/libraries/hanami_messages/protobuffers/hanami_messages.proto3.pb.h
RUN cmake -DCMAKE_BUILD_TYPE=Release .
RUN make -j8
RUN mkdir -p /app/ && \
    find src -type f -executable -exec cp {} /app/ \;


FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && \
    apt-get install -y openssl libuuid1 libcrypto++8 libsqlite3-0 libboost1.74 libprotobuf32t64 && \
    apt-get clean autoclean &&\
    apt-get autoremove --yes

# hanami
COPY --from=builder /app/hanami /usr/bin/hanami
CMD [ "hanami" ]
