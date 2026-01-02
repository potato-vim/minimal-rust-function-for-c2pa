//! # C2PA Primitives
//!
//! Minimal C2PA embedding for Rust functions with type-level provenance guarantees.
//!
//! The core idea: `C2pa<T, Verified>` can only be constructed through verified paths,
//! making provenance tracking a compile-time guarantee.

use sha2::{Digest, Sha256};
use std::marker::PhantomData;
use thiserror::Error;

// ============================================================================
// Marker Types - Type-level state encoding
// ============================================================================

/// Marker indicating the value has been cryptographically verified.
/// This type is uninhabited outside this crate, sealing construction.
#[derive(Debug, Clone, Copy)]
pub struct Verified(());

/// Marker for unverified state.
#[derive(Debug, Clone, Copy)]
pub struct Unverified;

// ============================================================================
// Core Type: C2pa<T, S>
// ============================================================================

/// A value `T` bound to C2PA provenance.
///
/// The type parameter `S` encodes verification state:
/// - `Verified`: Cryptographically verified, only constructible through trusted paths
/// - `Unverified`: Not yet verified
///
/// # Design Philosophy
///
/// This type makes provenance a first-class citizen of your type system.
/// You cannot accidentally create a `C2pa<T, Verified>` - it must flow from
/// a verified source or be created through a signed transformation.
#[derive(Debug, Clone)]
pub struct C2pa<T, S = Unverified> {
    payload: T,
    provenance: Provenance,
    _state: PhantomData<S>,
}

impl<T, S> C2pa<T, S> {
    /// Access the inner payload.
    #[inline]
    pub fn payload(&self) -> &T {
        &self.payload
    }

    /// Access provenance metadata.
    #[inline]
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// Consume and return the inner payload.
    #[inline]
    pub fn into_payload(self) -> T {
        self.payload
    }
}

impl<T> C2pa<T, Unverified> {
    /// Create an unverified C2PA-wrapped value.
    /// This is the only public constructor.
    pub fn new(payload: T, provenance: Provenance) -> Self {
        Self {
            payload,
            provenance,
            _state: PhantomData,
        }
    }
}

impl<T> C2pa<T, Verified> {
    /// Internal constructor for verified values.
    /// Not public - can only be created through verification or transformation.
    fn new_verified(payload: T, provenance: Provenance) -> Self {
        Self {
            payload,
            provenance,
            _state: PhantomData,
        }
    }
}

// ============================================================================
// Provenance - Cryptographic lineage tracking
// ============================================================================

/// Provenance metadata linking a value to its C2PA manifest.
#[derive(Debug, Clone)]
pub struct Provenance {
    /// Active manifest identifier (JUMBF URI).
    pub manifest_id: String,
    /// SHA-256 hash of the claim.
    pub claim_hash: ClaimHash,
    /// How the asset is bound to the manifest.
    pub asset_binding: AssetBinding,
    /// Parent references (for transformed assets).
    pub ingredients: Vec<IngredientRef>,
}

impl Provenance {
    /// Create root provenance (no parents).
    pub fn root(manifest_id: impl Into<String>, claim_hash: ClaimHash, binding: AssetBinding) -> Self {
        Self {
            manifest_id: manifest_id.into(),
            claim_hash,
            asset_binding: binding,
            ingredients: Vec::new(),
        }
    }

    /// Create derived provenance (with parent references).
    pub fn derived(
        manifest_id: impl Into<String>,
        claim_hash: ClaimHash,
        binding: AssetBinding,
        ingredients: Vec<IngredientRef>,
    ) -> Self {
        Self {
            manifest_id: manifest_id.into(),
            claim_hash,
            asset_binding: binding,
            ingredients,
        }
    }
}

/// SHA-256 claim hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimHash(pub [u8; 32]);

impl ClaimHash {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// How an asset is bound to its manifest.
#[derive(Debug, Clone)]
pub enum AssetBinding {
    /// Hash-based binding (most common).
    Hash(ContentHash),
    /// Box-based binding with offset/length (for embedded data).
    Box { offset: u64, length: u64, hash: ContentHash },
}

/// SHA-256 content hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn compute<T: AsRef<[u8]>>(data: T) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data.as_ref());
        Self(hasher.finalize().into())
    }
}

/// Reference to a parent ingredient.
#[derive(Debug, Clone)]
pub struct IngredientRef {
    /// Parent's claim hash.
    pub claim_hash: ClaimHash,
    /// Parent's asset binding.
    pub asset_binding: AssetBinding,
    /// Relationship type (e.g., "parentOf", "componentOf").
    pub relationship: IngredientRelation,
}

/// C2PA-defined ingredient relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IngredientRelation {
    ParentOf,
    ComponentOf,
    InputTo,
}

impl IngredientRelation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ParentOf => "parentOf",
            Self::ComponentOf => "componentOf",
            Self::InputTo => "inputTo",
        }
    }
}

// ============================================================================
// Trait: C2paBindable - Content that can be bound to a manifest
// ============================================================================

/// Types that can be bound to a C2PA manifest.
///
/// Implement this for your domain types to enable C2PA wrapping.
///
/// # Example
///
/// ```ignore
/// struct ImageData {
///     pixels: Vec<u8>,
///     width: u32,
///     height: u32,
/// }
///
/// impl C2paBindable for ImageData {
///     fn content_hash(&self) -> ContentHash {
///         ContentHash::compute(&self.pixels)
///     }
///
///     fn media_type(&self) -> &str {
///         "image/png"
///     }
/// }
/// ```
pub trait C2paBindable {
    /// Compute the content hash for asset binding.
    fn content_hash(&self) -> ContentHash;

    /// MIME type of the content.
    fn media_type(&self) -> &str {
        "application/octet-stream"
    }
}

// Built-in implementations for common types
impl C2paBindable for Vec<u8> {
    fn content_hash(&self) -> ContentHash {
        ContentHash::compute(self)
    }
}

impl C2paBindable for [u8] {
    fn content_hash(&self) -> ContentHash {
        ContentHash::compute(self)
    }
}

impl C2paBindable for String {
    fn content_hash(&self) -> ContentHash {
        ContentHash::compute(self.as_bytes())
    }

    fn media_type(&self) -> &str {
        "text/plain"
    }
}

impl C2paBindable for str {
    fn content_hash(&self) -> ContentHash {
        ContentHash::compute(self.as_bytes())
    }

    fn media_type(&self) -> &str {
        "text/plain"
    }
}

// Numeric primitives
macro_rules! impl_bindable_for_primitive {
    ($($ty:ty),*) => {
        $(
            impl C2paBindable for $ty {
                fn content_hash(&self) -> ContentHash {
                    ContentHash::compute(self.to_le_bytes())
                }
            }
        )*
    };
}

impl_bindable_for_primitive!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64);

// ============================================================================
// Trait: C2paTransform - Provenance-preserving transformations
// ============================================================================

/// A transformation that creates verified output from verified input.
///
/// This is the core mechanism for building provenance chains.
/// The output's manifest will reference the input as an ingredient.
///
/// # Type Safety
///
/// The signature enforces that:
/// - Input must already be verified
/// - Output is verified by the transformation's signature
/// - Provenance chain is automatically maintained
pub trait C2paTransform<I: C2paBindable, O: C2paBindable> {
    /// Perform the transformation.
    ///
    /// The implementation should:
    /// 1. Transform `I` into `O`
    /// 2. Create a new manifest referencing `I` as an ingredient
    /// 3. Sign the manifest
    /// 4. Return `C2pa<O, Verified>`
    fn transform(
        &self,
        input: &C2pa<I, Verified>,
        ctx: &mut TransformContext,
    ) -> Result<C2pa<O, Verified>, TransformError>;
}

/// Context for performing transformations.
#[derive(Debug)]
pub struct TransformContext {
    /// Generator label (e.g., "MyApp/1.0").
    pub generator: String,
    /// Whether to require trusted timestamps.
    pub require_timestamp: bool,
    /// Custom assertions to add.
    pub assertions: Vec<CustomAssertion>,
}

impl TransformContext {
    pub fn new(generator: impl Into<String>) -> Self {
        Self {
            generator: generator.into(),
            require_timestamp: false,
            assertions: Vec::new(),
        }
    }

    pub fn with_timestamp(mut self, require: bool) -> Self {
        self.require_timestamp = require;
        self
    }

    pub fn add_assertion(mut self, assertion: CustomAssertion) -> Self {
        self.assertions.push(assertion);
        self
    }
}

/// Custom assertion to embed in the manifest.
#[derive(Debug, Clone)]
pub struct CustomAssertion {
    pub label: String,
    pub data: Vec<u8>,
    pub mime_type: String,
}

impl CustomAssertion {
    pub fn json(label: impl Into<String>, json: &str) -> Self {
        Self {
            label: label.into(),
            data: json.as_bytes().to_vec(),
            mime_type: "application/json".into(),
        }
    }
}

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum TransformError {
    #[error("verification failed: {0}")]
    Verification(String),

    #[error("signing failed: {0}")]
    Signing(String),

    #[error("binding failed: {0}")]
    Binding(String),

    #[error("C2PA error: {0}")]
    C2pa(String),
}

// ============================================================================
// Verification - The gateway to Verified state
// ============================================================================

/// Verify an unverified C2PA value.
///
/// This is one of the only ways to obtain a `C2pa<T, Verified>`.
pub fn verify<T: C2paBindable>(
    value: C2pa<T, Unverified>,
    expected_hash: &ClaimHash,
) -> Result<C2pa<T, Verified>, TransformError> {
    // Verify the claim hash matches
    if &value.provenance.claim_hash != expected_hash {
        return Err(TransformError::Verification(
            "claim hash mismatch".into(),
        ));
    }

    // Verify asset binding
    let computed = value.payload.content_hash();
    match &value.provenance.asset_binding {
        AssetBinding::Hash(expected) if expected == &computed => {}
        AssetBinding::Box { hash, .. } if hash == &computed => {}
        _ => {
            return Err(TransformError::Verification(
                "asset binding mismatch".into(),
            ));
        }
    }

    Ok(C2pa::new_verified(value.payload, value.provenance))
}

// ============================================================================
// Builder - Simplified manifest creation (wraps c2pa crate)
// ============================================================================

/// Builder for creating verified C2PA values.
///
/// This provides a safe interface for creating `C2pa<T, Verified>` values
/// by ensuring proper signing.
pub struct C2paBuilder<T: C2paBindable> {
    payload: T,
    ingredients: Vec<IngredientRef>,
    generator: String,
}

impl<T: C2paBindable> C2paBuilder<T> {
    /// Start building a C2PA value.
    pub fn new(payload: T) -> Self {
        Self {
            payload,
            ingredients: Vec::new(),
            generator: "c2pa_primitives/0.1".into(),
        }
    }

    /// Set the generator label.
    pub fn generator(mut self, generator: impl Into<String>) -> Self {
        self.generator = generator.into();
        self
    }

    /// Add an ingredient reference from a verified source.
    pub fn add_ingredient<I: C2paBindable>(
        mut self,
        ingredient: &C2pa<I, Verified>,
        relation: IngredientRelation,
    ) -> Self {
        self.ingredients.push(IngredientRef {
            claim_hash: ingredient.provenance.claim_hash.clone(),
            asset_binding: ingredient.provenance.asset_binding.clone(),
            relationship: relation,
        });
        self
    }

    /// Sign and create a verified C2PA value.
    ///
    /// In a real implementation, this would use the c2pa crate's signing.
    /// For this prototype, we simulate the process.
    pub fn sign(self, _signer: &dyn Signer) -> Result<C2pa<T, Verified>, TransformError> {
        // Compute content hash
        let content_hash = self.payload.content_hash();
        let binding = AssetBinding::Hash(content_hash);

        // Simulate claim hash computation
        let claim_hash = self.compute_claim_hash(&binding);

        // Generate manifest ID
        let manifest_id = format!(
            "urn:uuid:{}",
            uuid_from_bytes(&claim_hash.0[..16])
        );

        let provenance = if self.ingredients.is_empty() {
            Provenance::root(manifest_id, claim_hash, binding)
        } else {
            Provenance::derived(manifest_id, claim_hash, binding, self.ingredients)
        };

        Ok(C2pa::new_verified(self.payload, provenance))
    }

    fn compute_claim_hash(&self, binding: &AssetBinding) -> ClaimHash {
        let mut hasher = Sha256::new();
        hasher.update(self.generator.as_bytes());

        if let AssetBinding::Hash(h) = binding {
            hasher.update(&h.0);
        }

        for ingredient in &self.ingredients {
            hasher.update(&ingredient.claim_hash.0);
        }

        ClaimHash(hasher.finalize().into())
    }
}

/// Minimal signer trait.
pub trait Signer {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, TransformError>;
    fn certificate_chain(&self) -> &[Vec<u8>];
}

/// Placeholder signer for prototyping.
pub struct TestSigner;

impl Signer for TestSigner {
    fn sign(&self, _data: &[u8]) -> Result<Vec<u8>, TransformError> {
        // Placeholder - would use real signing in production
        Ok(vec![0u8; 64])
    }

    fn certificate_chain(&self) -> &[Vec<u8>] {
        &[]
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn uuid_from_bytes(bytes: &[u8]) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    )
}

// ============================================================================
// Example: Function Transform
// ============================================================================

/// Transform that applies a function to the payload while preserving provenance.
///
/// This demonstrates how to create a type-safe transformation.
pub struct FnTransform<F, I, O>
where
    F: Fn(&I) -> O,
    I: C2paBindable,
    O: C2paBindable,
{
    func: F,
    _action_label: String,
    _phantom: PhantomData<(I, O)>,
}

impl<F, I, O> FnTransform<F, I, O>
where
    F: Fn(&I) -> O,
    I: C2paBindable,
    O: C2paBindable,
{
    pub fn new(func: F, action_label: impl Into<String>) -> Self {
        Self {
            func,
            _action_label: action_label.into(),
            _phantom: PhantomData,
        }
    }
}

impl<F, I, O> C2paTransform<I, O> for FnTransform<F, I, O>
where
    F: Fn(&I) -> O,
    I: C2paBindable,
    O: C2paBindable,
{
    fn transform(
        &self,
        input: &C2pa<I, Verified>,
        ctx: &mut TransformContext,
    ) -> Result<C2pa<O, Verified>, TransformError> {
        // Apply the transformation
        let output = (self.func)(input.payload());

        // Build with ingredient reference
        let builder = C2paBuilder::new(output)
            .generator(&ctx.generator)
            .add_ingredient(input, IngredientRelation::ParentOf);

        builder.sign(&TestSigner)
    }
}

// ============================================================================
// Convenience macros
// ============================================================================

/// Create a verified C2PA value from a payload (for trusted sources).
///
/// This is a "trust me" escape hatch for when you have verified content
/// from an external trusted source.
#[macro_export]
macro_rules! c2pa_trusted {
    ($payload:expr, $manifest_id:expr, $claim_hash:expr) => {{
        let payload = $payload;
        let hash = $crate::ContentHash::compute(&payload);
        let claim = $crate::ClaimHash::from_bytes($claim_hash);
        let prov = $crate::Provenance::root(
            $manifest_id,
            claim,
            $crate::AssetBinding::Hash(hash),
        );
        // SAFETY: This bypasses verification - use only for trusted sources
        $crate::C2paBuilder::new(payload)
            .sign(&$crate::TestSigner)
            .expect("signing should not fail for trusted content")
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verified_type_safety() {
        // Create a verified value
        let verified: C2pa<u32, Verified> = C2paBuilder::new(42u32)
            .generator("test")
            .sign(&TestSigner)
            .unwrap();

        assert_eq!(*verified.payload(), 42);
        assert!(!verified.provenance().manifest_id.is_empty());
    }

    #[test]
    fn test_transform_preserves_provenance() {
        // Create input
        let input: C2pa<u32, Verified> = C2paBuilder::new(10u32)
            .generator("test")
            .sign(&TestSigner)
            .unwrap();

        // Define transformation: multiply by 2
        let transform = FnTransform::new(|x: &u32| x * 2, "multiply");

        // Apply
        let mut ctx = TransformContext::new("test");
        let output: C2pa<u32, Verified> = transform.transform(&input, &mut ctx).unwrap();

        assert_eq!(*output.payload(), 20);
        assert_eq!(output.provenance().ingredients.len(), 1);
        assert_eq!(
            output.provenance().ingredients[0].claim_hash,
            input.provenance().claim_hash
        );
    }

    #[test]
    fn test_chain_of_transforms() {
        let v1: C2pa<u32, Verified> = C2paBuilder::new(1u32)
            .sign(&TestSigner)
            .unwrap();

        let add_one = FnTransform::new(|x: &u32| x + 1, "increment");
        let mut ctx = TransformContext::new("test");

        let v2 = add_one.transform(&v1, &mut ctx).unwrap();
        let v3 = add_one.transform(&v2, &mut ctx).unwrap();
        let v4 = add_one.transform(&v3, &mut ctx).unwrap();

        assert_eq!(*v4.payload(), 4);

        // Each step references its parent
        assert_eq!(v4.provenance().ingredients[0].claim_hash, v3.provenance().claim_hash);
        assert_eq!(v3.provenance().ingredients[0].claim_hash, v2.provenance().claim_hash);
        assert_eq!(v2.provenance().ingredients[0].claim_hash, v1.provenance().claim_hash);
    }

    #[test]
    fn test_unverified_cannot_become_verified_directly() {
        let unverified = C2pa::<u32, Unverified>::new(
            42,
            Provenance::root(
                "test",
                ClaimHash([0; 32]),
                AssetBinding::Hash(ContentHash([0; 32])),
            ),
        );

        // This demonstrates type safety:
        // unverified cannot be used where Verified is required
        // The following would not compile:
        // let _: C2pa<u32, Verified> = unverified;

        // Must go through verification
        let result = verify(unverified, &ClaimHash([0; 32]));
        // Will fail because content hash doesn't match
        assert!(result.is_err());
    }
}
