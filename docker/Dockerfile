FROM debian:stable-slim
EXPOSE 9944 9933 30300
COPY evercity-node .
VOLUME /var/run/evercity
CMD [ "/evercity-node", "--dev", "--rpc-external", "--ws-external", "--ws-port", "9944",  "--rpc-port", "9933", "--port", "30300", "--base-path", "/var/run/evercity" ]
