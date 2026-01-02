//! C2PA Primitives - Minimal DSL Demo
//!
//! Run with: cargo run

use c2pa_primitives::debug::{print_step, verify_chain};
use c2pa_primitives::*;

#[c2pa_pipeline(generator = "demo")]
fn main() -> Result<(), TransformError> {
    
    let step1 = start_c2pa()?;
    print_step("step1", &step1);
    let step2 = double_c2pa(&step1)?;
    print_step("step2", &step2);
    let step3 = add_ten_c2pa(&step2)?;
    print_step("step3", &step3);
    verify_chain(&step2, &step1, "step2");
    verify_chain(&step3, &step2, "step3");

    println!("  Final value: {}", step3.payload());
    Ok(())
}

#[c2pa_source]
fn start() -> u32 {
    10
}

#[c2pa_transform(name = "double")]
fn double(x: &u32) -> u32 {
    x * 3
}

#[c2pa_transform(name = "add_ten")]
fn add_ten(x: &u32) -> u32 {
    x + 7
}
