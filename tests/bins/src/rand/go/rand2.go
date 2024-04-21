package main

import (
	"crypto/rand"
	"fmt"
	"math/big"
)

func main() {
	nBig, err := rand.Int(rand.Reader, big.NewInt(1000))
	if err != nil {
		panic(err)
	}
	fmt.Println(nBig.Int64())
}
