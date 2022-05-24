FROM rust:latest

 WORKDIR /app

 COPY . .

 EXPOSE 8000
 ENV ROCKET_PORT=8000

 RUN if [ ! -f "Cargo.toml" ]; then cargo init . ; fi
 RUN cargo install --path .

 CMD ["/app/target/release/bbutnerv3-backend"]