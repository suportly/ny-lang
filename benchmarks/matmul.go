package main

import (
	"fmt"
	"time"
)

func matmul(a, b, c []int, n int) {
	for i := 0; i < n; i++ {
		for j := 0; j < n; j++ {
			sum := 0
			for k := 0; k < n; k++ {
				sum += a[i*n+k] * b[k*n+j]
			}
			c[i*n+j] = sum
		}
	}
}

func checksum(m []int) int64 {
	var t int64
	for _, v := range m {
		t += int64(v)
	}
	return t
}

func bench(n int) {
	a := make([]int, n*n)
	b := make([]int, n*n)
	c := make([]int, n*n)
	for i := 0; i < n; i++ {
		for j := 0; j < n; j++ {
			a[i*n+j] = (i + j) % 7
			b[i*n+j] = (i + j) % 5
		}
	}

	start := time.Now()
	matmul(a, b, c, n)
	elapsed := time.Since(start)

	fmt.Printf("  %dx%d: %dms (checksum: %d)\n", n, n, elapsed.Milliseconds(), checksum(c))
}

func main() {
	fmt.Println("=== Go Matrix Multiply Benchmark ===")
	fmt.Println()
	bench(32)
	bench(64)
	bench(128)
	bench(256)
	fmt.Println()
	fmt.Println("=== Done ===")
}
