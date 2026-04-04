package main

import (
	"fmt"
	"math"
	"time"
)

const (
	PI        = 3.141592653589793
	SolarMass = 4 * PI * PI
	DT        = 0.01
)

type Body struct{ x, y, z, vx, vy, vz, mass float64 }

var bodies = [5]Body{
	{0, 0, 0, 0, 0, 0, SolarMass},
	{4.84143144246472090, -1.16032004402742839, -0.103622044471123109,
		0.00166007664274403694 * 365.24, 0.00769901118419740425 * 365.24, -0.0000690460016972063023 * 365.24,
		0.000954791938424326609 * SolarMass},
	{8.34336671824457987, 4.12479856412430479, -0.403523417114321381,
		-0.00276742510726862411 * 365.24, 0.00499852801234917238 * 365.24, 0.0000230417297573763929 * 365.24,
		0.000285885980666130812 * SolarMass},
	{12.8943695621391310, -15.1111514016986312, -0.223307578892655734,
		0.00296460137564761618 * 365.24, 0.00237847173959480950 * 365.24, -0.0000296589568540237556 * 365.24,
		0.0000436624404335156298 * SolarMass},
	{15.3796971148509165, -25.9193146099879641, 0.179258772950371181,
		0.00268067772490389322 * 365.24, 0.00162824170038242295 * 365.24, -0.0000951592254519715870 * 365.24,
		0.0000515138902046611451 * SolarMass},
}

func energy() float64 {
	e := 0.0
	for i := 0; i < 5; i++ {
		e += 0.5 * bodies[i].mass * (bodies[i].vx*bodies[i].vx + bodies[i].vy*bodies[i].vy + bodies[i].vz*bodies[i].vz)
		for j := i + 1; j < 5; j++ {
			dx := bodies[i].x - bodies[j].x
			dy := bodies[i].y - bodies[j].y
			dz := bodies[i].z - bodies[j].z
			e -= bodies[i].mass * bodies[j].mass / math.Sqrt(dx*dx+dy*dy+dz*dz)
		}
	}
	return e
}

func advance() {
	for i := 0; i < 5; i++ {
		for j := i + 1; j < 5; j++ {
			dx := bodies[i].x - bodies[j].x
			dy := bodies[i].y - bodies[j].y
			dz := bodies[i].z - bodies[j].z
			d2 := dx*dx + dy*dy + dz*dz
			mag := DT / (d2 * math.Sqrt(d2))
			bodies[i].vx -= dx * bodies[j].mass * mag
			bodies[i].vy -= dy * bodies[j].mass * mag
			bodies[i].vz -= dz * bodies[j].mass * mag
			bodies[j].vx += dx * bodies[i].mass * mag
			bodies[j].vy += dy * bodies[i].mass * mag
			bodies[j].vz += dz * bodies[i].mass * mag
		}
	}
	for i := 0; i < 5; i++ {
		bodies[i].x += DT * bodies[i].vx
		bodies[i].y += DT * bodies[i].vy
		bodies[i].z += DT * bodies[i].vz
	}
}

func main() {
	n := 500000
	px, py, pz := 0.0, 0.0, 0.0
	for i := 0; i < 5; i++ {
		px += bodies[i].vx * bodies[i].mass
		py += bodies[i].vy * bodies[i].mass
		pz += bodies[i].vz * bodies[i].mass
	}
	bodies[0].vx = -px / SolarMass
	bodies[0].vy = -py / SolarMass
	bodies[0].vz = -pz / SolarMass
	fmt.Printf("%.9f\n", energy())
	start := time.Now()
	for i := 0; i < n; i++ { advance() }
	elapsed := time.Since(start)
	fmt.Printf("%.9f\n", energy())
	fmt.Printf("n-body (%d steps): %dms\n", n, elapsed.Milliseconds())
}
