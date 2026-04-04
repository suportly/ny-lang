package main

import (
	"fmt"
	"time"
)

func ackermann(m, n int) int {
	if m == 0 { return n + 1 }
	if n == 0 { return ackermann(m-1, 1) }
	return ackermann(m-1, ackermann(m, n-1))
}

func main() {
	start := time.Now()
	result := ackermann(3, 12)
	elapsed := time.Since(start)
	fmt.Printf("ackermann(3,12) = %d\ntime: %dms\n", result, elapsed.Milliseconds())
}
