// Sieve of Eratosthenes — count primes up to N

fn sieve(n: i32) -> i32 {
    // Use Vec<i32> as boolean flags (0=composite, 1=prime)
    flags :~ Vec<i32> = vec_new();
    i :~ i32 = 0;
    while i <= n {
        flags.push(1);
        i += 1;
    }
    flags.set(0, 0);
    flags.set(1, 0);

    i = 2;
    while i * i <= n {
        if flags.get(i) == 1 {
            j :~ i32 = i * i;
            while j <= n {
                flags.set(j, 0);
                j += i;
            }
        }
        i += 1;
    }

    count :~ i32 = 0;
    i = 2;
    while i <= n {
        if flags.get(i) == 1 {
            count += 1;
        }
        i += 1;
    }
    return count;
}

fn main() -> i32 {
    n := 10000000;
    start := clock_ms();
    count := sieve(n);
    elapsed := clock_ms() - start;
    println(f"primes up to {n}: {count}");
    println(f"time: {elapsed}ms");
    return 0;
}
