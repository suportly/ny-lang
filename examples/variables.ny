// Demonstrates Ny variable declarations and expression-body functions.
//
// Compile and run:
//   ny build variables.ny -o vars
//   ./vars
//   echo $?  # prints 70
// Expression-body function (concise single-expression form)
fn square(x: i32) -> i32 {
    return x * x;
}

fn abs(x: i32) -> i32 {
    if x < 0 {
        return -x;
    }
    return x;
}

fn main() -> i32 {
    // Immutable variable (cannot be reassigned)
    result : i32 = square(5);
    // Mutable variable (can be reassigned)
    sum :~ i32 = 0;
    i :~ i32 = 0;
    while i < 10 {
        sum += i;
        i += 1;
    }
    // result = 25, sum = 45 → returns 70
    return sum + result;
}
