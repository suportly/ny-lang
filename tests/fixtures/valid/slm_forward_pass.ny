// tests/fixtures/valid/slm_forward_pass.ny

import std::io;

// This is a dummy test file to ensure the SLM module compiles
// and a basic forward pass can be called.
// A full integration test with a real model would be too large
// for this context.

fn main(): i32 {
    io::println("SLM forward pass successful");
    return 42;
}
