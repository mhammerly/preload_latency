FROM rust:trixie

RUN apt-get update
RUN apt-get install -y protobuf-compiler strace
