// The simplest Ny program.
// Returns 42 as the process exit code.
//
// Compile and run:
//   ny build hello.ny
//   ./hello
//   echo $?  # prints 42
fn main() -> i32 {
    return 42;
}
