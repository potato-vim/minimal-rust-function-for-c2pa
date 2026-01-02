//! Integration tests for C2PA macros
//!
//! These tests verify macro-generated transform wrappers work correctly.

use c2pa_primitives::*;

// ============================================================================
// Phase 0: Basic transform tests
// ============================================================================

/// Basic transform: fn(&T) -> U
#[c2pa_transform(name = "triple", relationship = "derivedFrom")]
fn triple(x: &u32) -> u32 {
    x * 3
}

#[test]
fn test_macro_basic_transform() {
    with_new_ctx("test", || {
        let source: C2pa<u32, Verified> = C2paBuilder::new(7u32)
            .generator("test")
            .sign(&TestSigner)
            .unwrap();

        let result = triple_c2pa(&source).unwrap();

        // Value is correctly transformed
        assert_eq!(*result.payload(), 21);

        // Provenance has one ingredient (the source)
        assert_eq!(result.provenance().ingredients.len(), 1);

        // Ingredient references the source
        assert_eq!(
            result.provenance().ingredients[0].claim_hash,
            source.provenance().claim_hash
        );
    });
}

#[test]
fn test_macro_chain() {
    with_new_ctx("test", || {
        let v1: C2pa<u32, Verified> = C2paBuilder::new(2u32)
            .generator("test")
            .sign(&TestSigner)
            .unwrap();

        let v2 = triple_c2pa(&v1).unwrap();
        let v3 = triple_c2pa(&v2).unwrap();

        assert_eq!(*v3.payload(), 18); // 2 * 3 * 3

        // Chain is preserved
        assert_eq!(
            v3.provenance().ingredients[0].claim_hash,
            v2.provenance().claim_hash
        );
        assert_eq!(
            v2.provenance().ingredients[0].claim_hash,
            v1.provenance().claim_hash
        );
    });
}

// ============================================================================
// Phase 1: Transform with parameter commits
// ============================================================================

#[derive(Debug, Clone)]
struct Offset {
    dx: i32,
    dy: i32,
}

#[c2pa_transform(name = "shift", record(params(offset)))]
fn shift_value(x: &i32, offset: Offset) -> i32 {
    x + offset.dx + offset.dy
}

#[test]
fn test_macro_param_commit() {
    with_new_ctx("test", || {
        let source: C2pa<i32, Verified> = C2paBuilder::new(100i32)
            .generator("test")
            .sign(&TestSigner)
            .unwrap();

        let offset = Offset { dx: 5, dy: 10 };
        let result = shift_value_c2pa(&source, offset).unwrap();

        // Value is correctly transformed
        assert_eq!(*result.payload(), 115);

        // Provenance is tracked
        assert_eq!(result.provenance().ingredients.len(), 1);
    });
}

#[test]
fn test_macro_different_params_produce_different_results() {
    with_new_ctx("test", || {
        let source: C2pa<i32, Verified> = C2paBuilder::new(100i32)
            .generator("test")
            .sign(&TestSigner)
            .unwrap();

        let offset1 = Offset { dx: 5, dy: 10 };
        let result1 = shift_value_c2pa(&source, offset1).unwrap();

        let offset2 = Offset { dx: 99, dy: 99 };
        let result2 = shift_value_c2pa(&source, offset2).unwrap();

        // Different params produce different claim hashes
        assert_ne!(
            result1.provenance().claim_hash,
            result2.provenance().claim_hash
        );
    });
}

// ============================================================================
// Multiple parameters test
// ============================================================================

#[derive(Debug, Clone)]
struct Scale {
    factor: f64,
}

#[c2pa_transform(name = "transform_both", record(params(offset, scale)))]
fn transform_with_multiple(x: &i32, offset: Offset, scale: Scale) -> i32 {
    (((*x + offset.dx + offset.dy) as f64) * scale.factor) as i32
}

#[test]
fn test_macro_multiple_param_commits() {
    with_new_ctx("test", || {
        let source: C2pa<i32, Verified> = C2paBuilder::new(10i32)
            .generator("test")
            .sign(&TestSigner)
            .unwrap();

        let offset = Offset { dx: 5, dy: 5 };
        let scale = Scale { factor: 2.0 };
        let result = transform_with_multiple_c2pa(&source, offset, scale).unwrap();

        // (10 + 5 + 5) * 2.0 = 40
        assert_eq!(*result.payload(), 40);
    });
}

// ============================================================================
// Original function remains usable
// ============================================================================

#[test]
fn test_original_function_still_works() {
    // The original function should still be callable directly
    let value = 7u32;
    let result = triple(&value);
    assert_eq!(result, 21);

    // shift_value too
    let offset = Offset { dx: 1, dy: 2 };
    let result = shift_value(&100, offset);
    assert_eq!(result, 103);
}

// ============================================================================
// Relationship tests
// ============================================================================

#[c2pa_transform(name = "parent_test", relationship = "parentOf")]
fn parent_transform(x: &u32) -> u32 {
    x + 1
}

#[c2pa_transform(name = "component_test", relationship = "componentOf")]
fn component_transform(x: &u32) -> u32 {
    x + 2
}

#[test]
fn test_different_relationships() {
    with_new_ctx("test", || {
        let source: C2pa<u32, Verified> = C2paBuilder::new(10u32)
            .generator("test")
            .sign(&TestSigner)
            .unwrap();

        let result1 = parent_transform_c2pa(&source).unwrap();
        assert_eq!(
            result1.provenance().ingredients[0].relationship,
            IngredientRelation::ParentOf
        );

        let result2 = component_transform_c2pa(&source).unwrap();
        assert_eq!(
            result2.provenance().ingredients[0].relationship,
            IngredientRelation::ComponentOf
        );
    });
}

// ============================================================================
// c2pa_source tests
// ============================================================================

#[c2pa_source]
fn origin_value() -> u32 {
    42
}

#[test]
fn test_source_creates_verified() {
    with_new_ctx("test", || {
        let verified = origin_value_c2pa().unwrap();
        assert_eq!(*verified.payload(), 42);
        // Source has no ingredients (it's the root)
        assert_eq!(verified.provenance().ingredients.len(), 0);
    });
}

#[test]
fn test_source_and_transform_chain() {
    with_new_ctx("test", || {
        let start = origin_value_c2pa().unwrap();
        let result = triple_c2pa(&start).unwrap();

        assert_eq!(*result.payload(), 126); // 42 * 3
        assert_eq!(result.provenance().ingredients.len(), 1);
    });
}

// ============================================================================
// c2pa_pipeline tests
// ============================================================================

#[c2pa_pipeline(generator = "pipeline_test")]
fn run_pipeline() -> Result<u32, TransformError> {
    let start = origin_value_c2pa()?;
    let result = triple_c2pa(&start)?;
    Ok(*result.payload())
}

#[test]
fn test_pipeline_basic() {
    let result = run_pipeline().unwrap();
    assert_eq!(result, 126);
}

#[test]
#[should_panic(expected = "with_ctx called outside #[c2pa_pipeline]")]
fn test_transform_without_pipeline_panics() {
    // This should panic because there's no active pipeline context
    let source: C2pa<u32, Verified> = C2paBuilder::new(1u32)
        .generator("test")
        .sign(&TestSigner)
        .unwrap();
    let _ = triple_c2pa(&source);
}
