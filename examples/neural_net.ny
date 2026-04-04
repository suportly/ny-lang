// Neural Network Forward Pass — Demonstrates: Tensor operations for ML
// A simple 2-layer network: input(4) → hidden(3) → output(1)
//
// Usage: ny run neural_net.ny

fn sigmoid(x: f64) -> f64 {
    return 1.0 / (1.0 + exp(0.0 - x));
}

fn main() -> i32 {
    println("=== Ny Neural Network Demo ===");
    println("");

    // Input: 1x4 vector
    input := tensor_zeros(1, 4);
    defer tensor_free(input);
    tensor_set(input, 0, 0, 0.5);
    tensor_set(input, 0, 1, 0.8);
    tensor_set(input, 0, 2, 0.2);
    tensor_set(input, 0, 3, 0.9);

    // Weights layer 1: 4x3
    w1 := tensor_fill(4, 3, 0.1);
    defer tensor_free(w1);
    tensor_set(w1, 0, 0, 0.3);
    tensor_set(w1, 1, 1, 0.5);
    tensor_set(w1, 2, 2, 0.7);
    tensor_set(w1, 3, 0, 0.2);

    // Weights layer 2: 3x1
    w2 := tensor_fill(3, 1, 0.4);
    defer tensor_free(w2);

    // Forward pass: hidden = input @ w1
    start := clock_ms();

    hidden := tensor_matmul(input, w1);
    defer tensor_free(hidden);

    // Apply sigmoid activation
    i :~ i32 = 0;
    while i < 3 {
        val := tensor_get(hidden, 0, i);
        tensor_set(hidden, 0, i, sigmoid(val));
        i += 1;
    }

    // Output = hidden @ w2
    output := tensor_matmul(hidden, w2);
    defer tensor_free(output);

    result := tensor_get(output, 0, 0);
    prediction := sigmoid(result);

    elapsed := clock_ms() - start;

    println("Input:");
    tensor_print(input);
    println("");
    println("Hidden (after sigmoid):");
    tensor_print(hidden);
    println("");
    print("Prediction: ");
    println(float_to_str(prediction));
    println(f"Time: {elapsed}ms");

    println("");
    println("=== Done ===");
    return 0;
}
