package main

import (
	"fmt"
	"time"
)

func sieve(n int) int {
	flags := make([]bool, n+1)
	for i := range flags { flags[i] = true }
	flags[0], flags[1] = false, false
	for i := 2; i*i <= n; i++ {
		if flags[i] {
			for j := i * i; j <= n; j += i {
				flags[j] = false
			}
		}
	}
	count := 0
	for i := 2; i <= n; i++ {
		if flags[i] { count++ }
	}
	return count
}

func main() {
	n := 10000000
	start := time.Now()
	count := sieve(n)
	elapsed := time.Since(start)
	fmt.Printf("primes up to %d: %d\ntime: %dms\n", n, count, elapsed.Milliseconds())
}
