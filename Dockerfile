FROM debian:latest
WORKDIR /usr/local/bin
COPY ./target/release/cash_microservice /usr/local/bin/invoice_microservice
RUN apt-get update && apt-get install -y
RUN apt-get install curl -y
STOPSIGNAL SIGINT
ENTRYPOINT ["invoice_microservice"]