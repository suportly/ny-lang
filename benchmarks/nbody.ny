// N-Body simulation — gravitational physics benchmark
// Based on the Computer Language Benchmarks Game

fn main() -> i32 {
    pi := 3.141592653589793;
    solar_mass := 4.0 * pi * pi;
    dt := 0.01;
    n := 500000;

    // Bodies: Sun, Jupiter, Saturn, Uranus, Neptune
    // Stored as parallel arrays (7 properties per body)
    // x, y, z, vx, vy, vz, mass
    x :~ Vec<f64> = vec_new();
    y :~ Vec<f64> = vec_new();
    z :~ Vec<f64> = vec_new();
    vx :~ Vec<f64> = vec_new();
    vy :~ Vec<f64> = vec_new();
    vz :~ Vec<f64> = vec_new();
    mass :~ Vec<f64> = vec_new();

    // Sun
    x.push(0.0); y.push(0.0); z.push(0.0);
    vx.push(0.0); vy.push(0.0); vz.push(0.0);
    mass.push(solar_mass);

    // Jupiter
    x.push(4.84143144246472090); y.push(-1.16032004402742839); z.push(-0.103622044471123109);
    vx.push(0.00166007664274403694 * 365.24); vy.push(0.00769901118419740425 * 365.24); vz.push(-0.0000690460016972063023 * 365.24);
    mass.push(0.000954791938424326609 * solar_mass);

    // Saturn
    x.push(8.34336671824457987); y.push(4.12479856412430479); z.push(-0.403523417114321381);
    vx.push(-0.00276742510726862411 * 365.24); vy.push(0.00499852801234917238 * 365.24); vz.push(0.0000230417297573763929 * 365.24);
    mass.push(0.000285885980666130812 * solar_mass);

    // Uranus
    x.push(12.8943695621391310); y.push(-15.1111514016986312); z.push(-0.223307578892655734);
    vx.push(0.00296460137564761618 * 365.24); vy.push(0.00237847173959480950 * 365.24); vz.push(-0.0000296589568540237556 * 365.24);
    mass.push(0.0000436624404335156298 * solar_mass);

    // Neptune
    x.push(15.3796971148509165); y.push(-25.9193146099879641); z.push(0.179258772950371181);
    vx.push(0.00268067772490389322 * 365.24); vy.push(0.00162824170038242295 * 365.24); vz.push(-0.0000951592254519715870 * 365.24);
    mass.push(0.0000515138902046611451 * solar_mass);

    num_bodies := 5;

    // Offset momentum
    px :~ f64 = 0.0;
    py :~ f64 = 0.0;
    pz :~ f64 = 0.0;
    i :~ i32 = 0;
    while i < num_bodies {
        px += vx.get(i) * mass.get(i);
        py += vy.get(i) * mass.get(i);
        pz += vz.get(i) * mass.get(i);
        i += 1;
    }
    vx.set(0, 0.0 - px / solar_mass);
    vy.set(0, 0.0 - py / solar_mass);
    vz.set(0, 0.0 - pz / solar_mass);

    // Energy
    energy :~ f64 = 0.0;
    i = 0;
    while i < num_bodies {
        energy += 0.5 * mass.get(i) * (vx.get(i)*vx.get(i) + vy.get(i)*vy.get(i) + vz.get(i)*vz.get(i));
        j :~ i32 = i + 1;
        while j < num_bodies {
            dx := x.get(i) - x.get(j);
            dy := y.get(i) - y.get(j);
            dz := z.get(i) - z.get(j);
            dist := sqrt(dx*dx + dy*dy + dz*dz);
            energy -= mass.get(i) * mass.get(j) / dist;
            j += 1;
        }
        i += 1;
    }
    println(float_to_str(energy));

    // Advance
    start := clock_ms();
    step :~ i32 = 0;
    while step < n {
        i = 0;
        while i < num_bodies {
            j :~ i32 = i + 1;
            while j < num_bodies {
                dx := x.get(i) - x.get(j);
                dy := y.get(i) - y.get(j);
                dz := z.get(i) - z.get(j);
                d2 := dx*dx + dy*dy + dz*dz;
                dist := sqrt(d2);
                mag := dt / (d2 * dist);
                vx.set(i, vx.get(i) - dx * mass.get(j) * mag);
                vy.set(i, vy.get(i) - dy * mass.get(j) * mag);
                vz.set(i, vz.get(i) - dz * mass.get(j) * mag);
                vx.set(j, vx.get(j) + dx * mass.get(i) * mag);
                vy.set(j, vy.get(j) + dy * mass.get(i) * mag);
                vz.set(j, vz.get(j) + dz * mass.get(i) * mag);
                j += 1;
            }
            i += 1;
        }
        i = 0;
        while i < num_bodies {
            x.set(i, x.get(i) + dt * vx.get(i));
            y.set(i, y.get(i) + dt * vy.get(i));
            z.set(i, z.get(i) + dt * vz.get(i));
            i += 1;
        }
        step += 1;
    }
    elapsed := clock_ms() - start;

    // Final energy
    energy = 0.0;
    i = 0;
    while i < num_bodies {
        energy += 0.5 * mass.get(i) * (vx.get(i)*vx.get(i) + vy.get(i)*vy.get(i) + vz.get(i)*vz.get(i));
        j :~ i32 = i + 1;
        while j < num_bodies {
            dx := x.get(i) - x.get(j);
            dy := y.get(i) - y.get(j);
            dz := z.get(i) - z.get(j);
            dist := sqrt(dx*dx + dy*dy + dz*dz);
            energy -= mass.get(i) * mass.get(j) / dist;
            j += 1;
        }
        i += 1;
    }
    println(float_to_str(energy));
    println(f"n-body ({n} steps): {elapsed}ms");

    return 0;
}
