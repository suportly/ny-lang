package main

import (
	"fmt"
	"math"
	"time"
)

func evalA(i, j int) float64 { return 1.0 / float64((i+j)*(i+j+1)/2+i+1) }

func evalATimesU(u, v []float64, n int) {
	for i := 0; i < n; i++ {
		sum := 0.0
		for j := 0; j < n; j++ { sum += evalA(i, j) * u[j] }
		v[i] = sum
	}
}

func evalAtTimesU(u, v []float64, n int) {
	for i := 0; i < n; i++ {
		sum := 0.0
		for j := 0; j < n; j++ { sum += evalA(j, i) * u[j] }
		v[i] = sum
	}
}

func evalAtATimesU(u, v, tmp []float64, n int) {
	evalATimesU(u, tmp, n)
	evalAtTimesU(tmp, v, n)
}

func main() {
	n := 2000
	u := make([]float64, n)
	v := make([]float64, n)
	tmp := make([]float64, n)
	for i := range u { u[i] = 1.0 }
	start := time.Now()
	for i := 0; i < 10; i++ { evalAtATimesU(u, v, tmp, n); evalAtATimesU(v, u, tmp, n) }
	vBv, vv := 0.0, 0.0
	for i := 0; i < n; i++ { vBv += u[i] * v[i]; vv += v[i] * v[i] }
	elapsed := time.Since(start)
	fmt.Printf("%.9f\n", math.Sqrt(vBv/vv))
	fmt.Printf("spectral-norm (n=%d): %dms\n", n, elapsed.Milliseconds())
}
