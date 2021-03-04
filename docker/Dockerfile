FROM debian:stable-slim
EXPOSE 9944 9933 30300
COPY evercity-node .
VOLUME /var/lib/evercity_chain
# Warning. Don't run docker container under the root.
# Run docker under unprivileged user i.e evercity.
# Specify userid and group in docker run command line as
#   --user userid:group
# or `user` parameter in the docker-compose.yml
#   user: userid
#
CMD [ "/evercity-node", "--dev", "--rpc-external", "--ws-external", "--base-path", "/var/lib/evercity_chain" ]
