FROM rustlang/rust:nightly

WORKDIR /app

COPY . .

EXPOSE 8000
ENV ROCKET_PORT=8000

RUN cargo build --release

CMD ["./target/release/bbutnerv3-backend"]