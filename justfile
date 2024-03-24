test:
    cargo test

run *cmd:
    ./target/debug/intercept -- {{cmd}}