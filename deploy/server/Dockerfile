FROM ubuntu:24.04

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libgcc-s1 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /opt/laml
COPY bin/ /usr/local/bin/
COPY web/ ./web/
COPY licenses/ ./licenses/
COPY SOURCE.txt ./SOURCE.txt

ENV LAML_NO_UPDATE_CHECK=1
USER 10001:10001
EXPOSE 7878
ENTRYPOINT ["laml-ui", "serve"]
CMD ["--assets", "/opt/laml/web", "--bind", "0.0.0.0:7878"]
