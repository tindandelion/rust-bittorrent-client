FROM linuxserver/transmission:latest

RUN mkdir /downloads && chmod 777 /downloads
RUN mkdir /downloads/complete && chmod 777 /downloads/complete
RUN mkdir /watch && chmod 777 /watch

COPY war-and-peace/war-and-peace.txt /downloads/complete
COPY war-and-peace/war-and-peace.torrent /watch
