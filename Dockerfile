#----------------------------------
# Source image

FROM paritytech/ci-linux:production as source

LABEL org.label-schema.vendor="Evercity" \
      org.label-schema.name="Smart Sustainable Bond" \
      org.label-schema.description="Blockchain node, which is a main part of Evercity's Smart Sustainable Bond project" \
      org.label-schema.url="https://evercity.io" \
      org.label-schema.schema-version="1.0" \
      org.opencontainers.image.source="https://github.com/EvercityEcosystem/smart-sustainable-bond"

WORKDIR /home/source
COPY . .
RUN cargo build --release


#----------------------------------
# Runtime image

FROM ubuntu:20.04 as runtime

LABEL org.label-schema.vendor="Evercity" \
      org.label-schema.name="Smart Sustainable Bond" \
      org.label-schema.description="Blockchain node, which is a main part of Evercity's Smart Sustainable Bond project" \
      org.label-schema.url="https://evercity.io" \
      org.label-schema.schema-version="1.0" \
      org.opencontainers.image.source="https://github.com/EvercityEcosystem/smart-sustainable-bond"

ENV USER="node"

RUN apt update && \
    addgroup --gecos "" --gid 2000 $USER && \
    adduser --gecos "" --gid 2000 --shell /bin/sh --disabled-login --disabled-password $USER

USER $USER
WORKDIR /home/$USER

COPY --chown=$USER:$USER --from=source ["/home/source/target/release", "/home/$USER/"]
RUN mkdir /home/$USER/chain

EXPOSE 9944 9615 9933 30300
CMD ["/home/node/evercity-node", "--base-path", "/home/node/chain", "--dev", "--rpc-external", "--ws-external", "--rpc-cors", "all", "--port", "30300", "--rpc-port", "9933", "--ws-port", "9944"]
