//! Integration tests for #[c2pa_transform] macro
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
    let source: C2pa<u32, Verified> = C2paBuilder::new(7u32)
        .generator("test")
        .sign(&TestSigner)
        .unwrap();

    let mut ctx = TransformContext::new("test");
    let result = triple_c2pa(&source, &mut ctx).unwrap();

    // Value is correctly transformed
    assert_eq!(*result.payload(), 21);

    // Provenance has one ingredient (the source)
    assert_eq!(result.provenance().ingredients.len(), 1);

    // Ingredient references the source
    assert_eq!(
        result.provenance().ingredients[0].claim_hash,
        source.provenance().claim_hash
    );

    // Transform name is recorded in context
    assert_eq!(ctx.transform_name, Some("triple".to_string()));
}

#[test]
fn test_macro_chain() {
    // Test that macro-generated transforms can be chained
    let v1: C2pa<u32, Verified> = C2paBuilder::new(2u32)
        .generator("test")
        .sign(&TestSigner)
        .unwrap();

    let mut ctx = TransformContext::new("test");
    let v2 = triple_c2pa(&v1, &mut ctx).unwrap();
    ctx.clear_transform_metadata();
    let v3 = triple_c2pa(&v2, &mut ctx).unwrap();

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
    let source: C2pa<i32, Verified> = C2paBuilder::new(100i32)
        .generator("test")
        .sign(&TestSigner)
        .unwrap();

    let offset = Offset { dx: 5, dy: 10 };
    let mut ctx = TransformContext::new("test");
    let result = shift_value_c2pa(&source, offset.clone(), &mut ctx).unwrap();

    // Value is correctly transformed
    assert_eq!(*result.payload(), 115);

    // Param commit is recorded
    assert_eq!(ctx.param_commits.len(), 1);
    assert_eq!(ctx.param_commits[0].0, "offset");

    // Commit is deterministic (same input = same hash)
    let offset2 = Offset { dx: 5, dy: 10 };
    let mut ctx2 = TransformContext::new("test");
    let _ = shift_value_c2pa(&source, offset2, &mut ctx2).unwrap();

    assert_eq!(ctx.param_commits[0].1, ctx2.param_commits[0].1);
}

#[test]
fn test_macro_param_commit_different_values() {
    let source: C2pa<i32, Verified> = C2paBuilder::new(100i32)
        .generator("test")
        .sign(&TestSigner)
        .unwrap();

    let offset1 = Offset { dx: 5, dy: 10 };
    let mut ctx1 = TransformContext::new("test");
    let _ = shift_value_c2pa(&source, offset1, &mut ctx1).unwrap();

    let offset2 = Offset { dx: 99, dy: 99 };
    let mut ctx2 = TransformContext::new("test");
    let _ = shift_value_c2pa(&source, offset2, &mut ctx2).unwrap();

    // Different values produce different commits
    assert_ne!(ctx1.param_commits[0].1, ctx2.param_commits[0].1);
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
    let source: C2pa<i32, Verified> = C2paBuilder::new(10i32)
        .generator("test")
        .sign(&TestSigner)
        .unwrap();

    let offset = Offset { dx: 5, dy: 5 };
    let scale = Scale { factor: 2.0 };
    let mut ctx = TransformContext::new("test");
    let result = transform_with_multiple_c2pa(&source, offset, scale, &mut ctx).unwrap();

    // (10 + 5 + 5) * 2.0 = 40
    assert_eq!(*result.payload(), 40);

    // Both parameters are committed
    assert_eq!(ctx.param_commits.len(), 2);
    assert_eq!(ctx.param_commits[0].0, "offset");
    assert_eq!(ctx.param_commits[1].0, "scale");
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
    let source: C2pa<u32, Verified> = C2paBuilder::new(10u32)
        .generator("test")
        .sign(&TestSigner)
        .unwrap();

    let mut ctx1 = TransformContext::new("test");
    let result1 = parent_transform_c2pa(&source, &mut ctx1).unwrap();
    assert_eq!(
        result1.provenance().ingredients[0].relationship,
        IngredientRelation::ParentOf
    );

    let mut ctx2 = TransformContext::new("test");
    let result2 = component_transform_c2pa(&source, &mut ctx2).unwrap();
    assert_eq!(
        result2.provenance().ingredients[0].relationship,
        IngredientRelation::ComponentOf
    );
}
