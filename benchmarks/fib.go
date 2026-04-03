package main

import (
	"fmt"
	"time"
)

func fibonacci(n int) int {
	if n <= 1 {
		return n
	}
	return fibonacci(n-1) + fibonacci(n-2)
}

func main() {
	fmt.Println("=== Go Fibonacci Benchmark ===")
	fmt.Println()

	start := time.Now()
	r35 := fibonacci(35)
	t35 := time.Since(start)
	fmt.Printf("  fibonacci(35) = %d\n  Time: %dms\n\n", r35, t35.Milliseconds())

	start = time.Now()
	r40 := fibonacci(40)
	t40 := time.Since(start)
	fmt.Printf("  fibonacci(40) = %d\n  Time: %dms\n\n", r40, t40.Milliseconds())

	fmt.Println("=== Done ===")
}
