build-bins:
    cd tests/bins/ && gcc -o rand_c_1 src/rand/c/rand1.c
    cd tests/bins/ && gcc -o rand_c_2 src/rand/c/rand2.c
    cd tests/bins/ && go build -o rand_go_1 src/rand/go/rand1.go
    cd tests/bins/ && go build -o rand_go_2 src/rand/go/rand2.go

test: build-bins
    cargo test

run *cmd:
    ./target/debug/intercept -- {{cmd}}