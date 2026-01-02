//! C2PA Primitives - Minimal DSL Demo
//!
//! Run with: cargo run

use c2pa_primitives::debug::{print_step, verify_chain};
use c2pa_primitives::*;

#[c2pa_pipeline(generator = "demo")]
fn main() -> Result<(), TransformError> {
    println!("═══════════════════════════════════════");
    println!("  C2PA Provenance Chain Demo");
    println!("═══════════════════════════════════════");

    // Step 1: Source
    let step1 = start_c2pa()?;
    print_step("Step 1: start() → 5", &step1);

    // Step 2: Double
    let step2 = double_c2pa(&step1)?;
    print_step("Step 2: double(5) → 10", &step2);

    // Step 3: Add ten
    let step3 = add_ten_c2pa(&step2)?;
    print_step("Step 3: add_ten(10) → 20", &step3);

    // Verify provenance chain
    println!("\n═══════════════════════════════════════");
    println!("  Chain Verification");
    println!("═══════════════════════════════════════");
    verify_chain(&step2, &step1, "step2");
    verify_chain(&step3, &step2, "step3");

    // Summary
    println!("\n═══════════════════════════════════════");
    println!("  Summary");
    println!("═══════════════════════════════════════");
    println!("  Final value: {}", step3.payload());
    println!("  Chain depth: 3 (start → double → add_ten)");
    println!("  All hashes linked: ✓");

    Ok(())
}

#[c2pa_source]
fn start() -> u32 {
    5
}

#[c2pa_transform(name = "double")]
fn double(x: &u32) -> u32 {
    x * 2
}

#[c2pa_transform(name = "add_ten")]
fn add_ten(x: &u32) -> u32 {
    x + 10
}
