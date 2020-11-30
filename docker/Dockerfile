FROM debian:stable-slim
EXPOSE 9944 9933 30300
COPY evercity-node .
VOLUME /var/run/evercity
# Warning. Don't run docker container under the root.
# Run docker under unprivileged user i.e evercity.
# Specify userid and group in docker run command line as
#   --user userid:group
# or `user` parameter in the docker-compose.yml
#   user: userid
#
CMD [ "/evercity-node", "--dev", "--rpc-external", "--ws-external", "--ws-port", "9944",  "--rpc-port", "9933", "--port", "30300", "--base-path", "/var/run/evercity" ]
