FROM fedora:33
RUN dnf update -y && dnf clean all -y
WORKDIR /usr/local/bin
COPY ./target/release/invoice_microservice /usr/local/bin/invoice_microservice
RUN dnf install curl -y && dnf clean all -y
STOPSIGNAL SIGINT
ENV RUST_LOG=trace
ENTRYPOINT ["invoice_microservice"]
