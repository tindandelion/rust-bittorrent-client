FROM linuxserver/transmission:latest

RUN mkdir /downloads && chmod 777 /downloads
RUN mkdir /watch && chmod 777 /watch

COPY debian-12.11.0-amd64-netinst.iso.torrent /watch
