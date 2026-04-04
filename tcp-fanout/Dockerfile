FROM golang:1.23-alpine AS build

WORKDIR /app
COPY main.go .
RUN go build -o proxy main.go

FROM alpine:3.19
WORKDIR /app
COPY --from=build /app/proxy .
CMD ["./proxy"]
