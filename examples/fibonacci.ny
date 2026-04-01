// Recursive Fibonacci — the Ny benchmark program.
// Computes fibonacci(10) = 55.
//
// Compile and run:
//   ny build fibonacci.ny -O 2 -o fib
//   ./fib
//   echo $?  # prints 55

fn fibonacci(n: i32) -> i32 {
    if n <= 1 {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

fn main() -> i32 {
    return fibonacci(10);
}
