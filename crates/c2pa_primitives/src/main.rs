//! C2PA Primitives - Minimal DSL Demo
//!
//! Run with: cargo run

use c2pa_primitives::*;

#[c2pa_pipeline(generator = "demo")]
fn main() -> Result<(), TransformError> {
    let start = start_c2pa()?;
    let result = add_ten_c2pa(&double_c2pa(&start)?)?;

    println!("{}", result.payload());
    println!("ingredients = {}", result.provenance().ingredients.len());
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
