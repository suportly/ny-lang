// Spectral Norm — iterative eigenvalue approximation

fn eval_A(i: i32, j: i32) -> f64 {
    return 1.0 / ((i + j) * (i + j + 1) / 2 + i + 1) as f64;
}

fn eval_A_times_u(u: Vec<f64>, v: Vec<f64>, n: i32) {
    i :~ i32 = 0;
    while i < n {
        sum :~ f64 = 0.0;
        j :~ i32 = 0;
        while j < n {
            sum += eval_A(i, j) * u.get(j);
            j += 1;
        }
        v.set(i, sum);
        i += 1;
    }
}

fn eval_At_times_u(u: Vec<f64>, v: Vec<f64>, n: i32) {
    i :~ i32 = 0;
    while i < n {
        sum :~ f64 = 0.0;
        j :~ i32 = 0;
        while j < n {
            sum += eval_A(j, i) * u.get(j);
            j += 1;
        }
        v.set(i, sum);
        i += 1;
    }
}

fn eval_AtA_times_u(u: Vec<f64>, v: Vec<f64>, tmp: Vec<f64>, n: i32) {
    eval_A_times_u(u, tmp, n);
    eval_At_times_u(tmp, v, n);
}

fn main() -> i32 {
    n := 2000;
    start := clock_ms();

    u :~ Vec<f64> = vec_new();
    v :~ Vec<f64> = vec_new();
    tmp :~ Vec<f64> = vec_new();

    i :~ i32 = 0;
    while i < n {
        u.push(1.0);
        v.push(0.0);
        tmp.push(0.0);
        i += 1;
    }

    iter :~ i32 = 0;
    while iter < 10 {
        eval_AtA_times_u(u, v, tmp, n);
        eval_AtA_times_u(v, u, tmp, n);
        iter += 1;
    }

    vBv :~ f64 = 0.0;
    vv :~ f64 = 0.0;
    i = 0;
    while i < n {
        vBv += u.get(i) * v.get(i);
        vv += v.get(i) * v.get(i);
        i += 1;
    }

    result := sqrt(vBv / vv);
    elapsed := clock_ms() - start;

    println(float_to_str(result));
    println(f"spectral-norm (n={n}): {elapsed}ms");
    return 0;
}
