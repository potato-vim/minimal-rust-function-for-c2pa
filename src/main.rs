//! C2PA Primitives - Essential Demos
//!
//! Run with: cargo run

use c2pa_primitives::*;

fn main() -> Result<(), TransformError> {
    println!("C2PA Primitives: Type-Safe Provenance\n");

    demo_chain()?;
    demo_verified_gate()?;
    demo_redaction()?;
    demo_graph()?;

    Ok(())
}

/// Existing demo: Chain of transforms (A → B → C)
fn demo_chain() -> Result<(), TransformError> {
    println!("═══ Demo 0: Provenance Chain ═══");
    println!("Shows: Verified → transform → Verified with parent tracking\n");

    let source: C2pa<u32, Verified> = C2paBuilder::new(10u32)
        .generator("demo/1.0")
        .sign(&TestSigner)?;

    let double = FnTransform::new(|x: &u32| x * 2, "double");
    let add_ten = FnTransform::new(|x: &u32| x + 10, "add_ten");
    let mut ctx = TransformContext::new("demo/1.0");

    let step1 = double.transform(&source, &mut ctx)?;
    let step2 = add_ten.transform(&step1, &mut ctx)?;

    println!("Chain: {} → {} → {}", source.payload(), step1.payload(), step2.payload());
    println!("Each step has 1 ingredient (parent):");
    println!("  step1.ingredients.len() = {}", step1.provenance().ingredients.len());
    println!("  step2.ingredients.len() = {}", step2.provenance().ingredients.len());
    println!();

    Ok(())
}

/// Demo 1: Verified Gate - Unverified bytes cannot be parsed
fn demo_verified_gate() -> Result<(), TransformError> {
    println!("═══ Demo 1: Verified Gate (Parse) ═══");
    println!("Shows: Unverified bytes → verify() → Verified bytes → parse → Invoice\n");

    // Simulate external input (e.g., from network)
    let raw_bytes = b"42:1000".to_vec();
    let content_hash = ContentHash::compute(&raw_bytes);

    // Create "unverified" wrapper (simulating received data with manifest)
    let claim_hash = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"external-source");
        h.update(&content_hash.0);
        ClaimHash(h.finalize().into())
    };

    let unverified: C2pa<Vec<u8>, Unverified> = C2pa::new(
        raw_bytes,
        Provenance::root(
            "urn:uuid:external-source",
            claim_hash.clone(),
            AssetBinding::Hash(content_hash),
        ),
    );

    println!("1. Received unverified bytes: {:?}",
             String::from_utf8_lossy(unverified.payload()));

    // This would NOT compile:
    // let parse = ParseTransform::<Invoice>::new();
    // parse.transform(&unverified, &mut ctx);  // ERROR: expected Verified

    println!("2. Cannot parse directly (compile error if tried)");

    // Must verify first
    let verified: C2pa<Vec<u8>, Verified> = verify(unverified, &claim_hash)?;
    println!("3. After verify(): now Verified");

    // Now we can parse
    let parse = ParseTransform::<Invoice>::new();
    let mut ctx = TransformContext::new("demo/1.0");
    let invoice: C2pa<Invoice, Verified> = parse.transform(&verified, &mut ctx)?;

    println!("4. Parsed invoice: {:?}", invoice.payload());
    println!("   Provenance: {} ingredient(s), relationship: {}",
             invoice.provenance().ingredients.len(),
             invoice.provenance().ingredients[0].relationship.as_str());
    println!();

    Ok(())
}

/// Demo 2: Redaction - Derived content maintains provenance
fn demo_redaction() -> Result<(), TransformError> {
    println!("═══ Demo 2: Redaction (Derivative) ═══");
    println!("Shows: Image → redact region → new Image with derivedFrom\n");

    // Create source image
    let source_img = Image::test_pattern(8, 4);
    let source: C2pa<Image, Verified> = C2paBuilder::new(source_img)
        .generator("camera/1.0")
        .sign(&TestSigner)?;

    println!("1. Source image: {}x{}", source.payload().width, source.payload().height);
    print_image(source.payload());

    // Redact region (2,1) to (5,2)
    let redact = RedactTransform::new(2, 1, 4, 2);
    let mut ctx = TransformContext::new("editor/1.0");
    let redacted: C2pa<Image, Verified> = redact.transform(&source, &mut ctx)?;

    println!("2. After redact(2,1,4,2):");
    print_image(redacted.payload());

    println!("3. Provenance preserved:");
    println!("   ingredients.len() = {}", redacted.provenance().ingredients.len());
    println!("   relationship = {}",
             redacted.provenance().ingredients[0].relationship.as_str());
    println!("   source hash = {:02x}{:02x}...",
             source.provenance().claim_hash.0[0],
             source.provenance().claim_hash.0[1]);
    println!("   ingredient hash = {:02x}{:02x}...",
             redacted.provenance().ingredients[0].claim_hash.0[0],
             redacted.provenance().ingredients[0].claim_hash.0[1]);
    println!();

    Ok(())
}

/// Demo 3: Graph - Multiple sources create DAG provenance
fn demo_graph() -> Result<(), TransformError> {
    println!("═══ Demo 3: Graph (DAG Provenance) ═══");
    println!("Shows: Image A + Image B → composite → new Image with 2 ingredients\n");

    // Create two independent source images
    let img_a = Image::new(4, 3, 0xAA);
    let img_b = Image::new(4, 3, 0x55);

    let source_a: C2pa<Image, Verified> = C2paBuilder::new(img_a)
        .generator("camera-a/1.0")
        .sign(&TestSigner)?;

    let source_b: C2pa<Image, Verified> = C2paBuilder::new(img_b)
        .generator("camera-b/1.0")
        .sign(&TestSigner)?;

    println!("1. Source A (4x3, fill=0xAA):");
    print_image(source_a.payload());

    println!("2. Source B (4x3, fill=0x55):");
    print_image(source_b.payload());

    // Compose: horizontal concatenation
    let concat = HConcatTransform;
    let mut ctx = TransformContext::new("compositor/1.0");
    let composite: C2pa<Image, Verified> = concat.compose(&source_a, &source_b, &mut ctx)?;

    println!("3. Composite (A | B):");
    print_image(composite.payload());

    println!("4. DAG Provenance:");
    println!("   ingredients.len() = {} (not 1, but 2!)",
             composite.provenance().ingredients.len());

    for (i, ing) in composite.provenance().ingredients.iter().enumerate() {
        println!("   [{}] {} → {:02x}{:02x}...",
                 i,
                 ing.relationship.as_str(),
                 ing.claim_hash.0[0],
                 ing.claim_hash.0[1]);
    }

    println!("\n   This is a DAG, not a chain:");
    println!("       A ─┐");
    println!("          ├─→ Composite");
    println!("       B ─┘");
    println!();

    Ok(())
}

/// Helper: print image as ASCII
fn print_image(img: &Image) {
    for y in 0..img.height {
        print!("   ");
        for x in 0..img.width {
            let v = img.get(x, y).unwrap_or(0);
            let c = if v == 0 { '░' } else if v < 0x80 { '▒' } else { '█' };
            print!("{}", c);
        }
        println!();
    }
}
