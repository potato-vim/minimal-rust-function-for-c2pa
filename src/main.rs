//! C2PA Primitives - Minimal Demonstration
//!
//! Run with: cargo run
//! Examples: cargo run --example image_pipeline
//!           cargo run --example primitive_functions

use c2pa_primitives::*;

fn main() -> Result<(), TransformError> {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           C2PA Primitives - Type-Safe Provenance             ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // ========================================================================
    // Core Concept: Verified values can only come from trusted sources
    // ========================================================================

    println!("┌─ Core Concept ──────────────────────────────────────────────┐");
    println!("│ C2pa<T, Verified> can only be created through:             │");
    println!("│   1. C2paBuilder::sign() - signing new content             │");
    println!("│   2. verify() - verifying existing content                 │");
    println!("│   3. C2paTransform::transform() - transforming verified    │");
    println!("└──────────────────────────────────────────────────────────────┘");
    println!();

    // ========================================================================
    // Demo 1: Creating verified values
    // ========================================================================

    println!("─── Demo 1: Creating Verified Values ───");

    // The ONLY way to get C2pa<T, Verified>
    let verified_number: C2pa<u32, Verified> = C2paBuilder::new(42u32)
        .generator("demo/1.0")
        .sign(&TestSigner)?;

    println!("Created: C2pa<u32, Verified>");
    println!("  Payload: {}", verified_number.payload());
    println!("  Manifest: {}", verified_number.provenance().manifest_id);
    println!();

    // ========================================================================
    // Demo 2: Type safety - functions requiring verified input
    // ========================================================================

    println!("─── Demo 2: Type-Safe Function Signatures ───");

    fn process_verified(input: &C2pa<u32, Verified>) -> u32 {
        // This function ONLY accepts verified values
        // The compiler ensures this at build time
        *input.payload() * 2
    }

    let result = process_verified(&verified_number);
    println!("process_verified({}) = {}", verified_number.payload(), result);

    // This would NOT compile:
    // let unverified = C2pa::<u32, Unverified>::new(...);
    // process_verified(&unverified);  // ERROR: expected Verified, found Unverified
    println!("  (Unverified values cannot be passed - compile error!)");
    println!();

    // ========================================================================
    // Demo 3: Transformations maintain provenance chain
    // ========================================================================

    println!("─── Demo 3: Provenance-Preserving Transforms ───");

    let double = FnTransform::new(|x: &u32| x * 2, "double");
    let add_ten = FnTransform::new(|x: &u32| x + 10, "add_ten");

    let mut ctx = TransformContext::new("demo/1.0");

    let step1 = double.transform(&verified_number, &mut ctx)?;
    let step2 = add_ten.transform(&step1, &mut ctx)?;

    println!("Chain: {} → {} → {}",
        verified_number.payload(),
        step1.payload(),
        step2.payload());

    println!("Provenance links:");
    println!("  step2 references step1: ✓ ({} parent)",
        step2.provenance().ingredients.len());
    println!("  step1 references source: ✓ ({} parent)",
        step1.provenance().ingredients.len());
    println!();

    // ========================================================================
    // Demo 4: The beauty - compile-time provenance guarantees
    // ========================================================================

    println!("─── Demo 4: Compile-Time Guarantees ───");
    println!();
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│ What the type system prevents:                             │");
    println!("│                                                             │");
    println!("│  ✗ Creating fake Verified values                           │");
    println!("│  ✗ Passing unverified data to verified-only functions     │");
    println!("│  ✗ Transforming unverified inputs                          │");
    println!("│  ✗ Breaking the provenance chain                           │");
    println!("│                                                             │");
    println!("│ All enforced at COMPILE TIME - zero runtime cost!          │");
    println!("└─────────────────────────────────────────────────────────────┘");
    println!();

    // ========================================================================
    // Demo 5: The provenance structure
    // ========================================================================

    println!("─── Demo 5: Inspecting Provenance ───");

    let prov = step2.provenance();
    println!("Manifest ID: {}", prov.manifest_id);
    println!("Claim Hash:  {:02x}{:02x}{:02x}{:02x}...",
        prov.claim_hash.0[0], prov.claim_hash.0[1],
        prov.claim_hash.0[2], prov.claim_hash.0[3]);
    println!("Ingredients: {}", prov.ingredients.len());

    for (i, ing) in prov.ingredients.iter().enumerate() {
        println!("  [{}] {:?} → {:02x}{:02x}...",
            i,
            ing.relationship.as_str(),
            ing.claim_hash.0[0],
            ing.claim_hash.0[1]);
    }
    println!();

    println!("═══════════════════════════════════════════════════════════════");
    println!("Run examples for more:");
    println!("  cargo run --example image_pipeline");
    println!("  cargo run --example primitive_functions");
    println!("═══════════════════════════════════════════════════════════════");

    Ok(())
}
