// Benchmark: concurrent sum via goroutines + channels (Go equivalent)
// go build -o concurrent_sum_go benchmarks/concurrent_sum.go && ./concurrent_sum_go

package main

import (
	"fmt"
	"time"
)

func worker(ch chan int64, start, end int64) {
	var total int64
	for i := start; i < end; i++ {
		total += i
	}
	ch <- total
}

func main() {
	var n int64 = 100_000_000
	var numWorkers int64 = 8
	chunk := n / numWorkers

	ch := make(chan int64, 16)

	start := time.Now()

	for i := int64(0); i < numWorkers; i++ {
		lo := i * chunk
		hi := lo + chunk
		go worker(ch, lo, hi)
	}

	var total int64
	for i := int64(0); i < numWorkers; i++ {
		total += <-ch
	}

	elapsed := time.Since(start)
	fmt.Printf("sum(0..%d) = %d\n", n, total)
	fmt.Printf("workers: %d\n", numWorkers)
	fmt.Printf("time: %dms\n", elapsed.Milliseconds())
}
