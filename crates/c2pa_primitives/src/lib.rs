//! # C2PA Primitives
//!
//! Minimal C2PA embedding for Rust functions with type-level provenance guarantees.
//!
//! The core idea: `C2pa<T, Verified>` can only be constructed through verified paths,
//! making provenance tracking a compile-time guarantee.
//!
//! ## Attribute Macro
//!
//! Use `#[c2pa_transform]` to automatically generate provenance-preserving wrappers:
//!
//! ```ignore
//! use c2pa_primitives::c2pa_transform;
//!
//! #[c2pa_transform(name = "double")]
//! fn double(x: &u32) -> u32 {
//!     x * 2
//! }
//!
//! // Now you can use `double_c2pa(&verified_input, &mut ctx)`
//! ```

use sha2::{Digest, Sha256};
use std::marker::PhantomData;
use thiserror::Error;

// Re-export the attribute macros
pub use c2pa_macros::{c2pa_pipeline, c2pa_source, c2pa_transform};

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
    /// Used when content is derived/transformed from source
    DerivedFrom,
    /// Used when multiple sources are composed together
    ComposedFrom,
}

impl IngredientRelation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ParentOf => "parentOf",
            Self::ComponentOf => "componentOf",
            Self::InputTo => "inputTo",
            Self::DerivedFrom => "derivedFrom",
            Self::ComposedFrom => "composedFrom",
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
    /// Transform name (set by macro-generated code).
    pub transform_name: Option<String>,
    /// Parameter commits (name -> hash). Values are NOT stored.
    pub param_commits: Vec<(String, [u8; 32])>,
}

impl TransformContext {
    pub fn new(generator: impl Into<String>) -> Self {
        Self {
            generator: generator.into(),
            require_timestamp: false,
            assertions: Vec::new(),
            transform_name: None,
            param_commits: Vec::new(),
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

    /// Set the transform name (used by macro-generated code).
    #[doc(hidden)]
    pub fn set_transform_name(&mut self, name: &str) {
        self.transform_name = Some(name.to_string());
    }

    /// Add a parameter commit (used by macro-generated code).
    /// Only the hash is stored, NOT the raw value.
    #[doc(hidden)]
    pub fn add_param_commit(&mut self, name: String, commit: [u8; 32]) {
        self.param_commits.push((name, commit));
    }

    /// Clear transform metadata for reuse.
    pub fn clear_transform_metadata(&mut self) {
        self.transform_name = None;
        self.param_commits.clear();
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
    assertions: Vec<CustomAssertion>,
}

impl<T: C2paBindable> C2paBuilder<T> {
    /// Start building a C2PA value.
    pub fn new(payload: T) -> Self {
        Self {
            payload,
            ingredients: Vec::new(),
            generator: "c2pa_primitives/0.1".into(),
            assertions: Vec::new(),
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

    /// Add a custom assertion to the manifest.
    pub fn add_assertion(mut self, assertion: CustomAssertion) -> Self {
        self.assertions.push(assertion);
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

        // Simulate claim hash computation (includes assertions)
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

        // Include assertions in claim hash
        for assertion in &self.assertions {
            hasher.update(assertion.label.as_bytes());
            hasher.update(&assertion.data);
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
// Demo Domain Types
// ============================================================================

/// A simple invoice for Demo 1 (Verified Gate Parse).
///
/// This type can only be parsed from verified bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct Invoice {
    pub id: u32,
    pub amount: u32,
}

impl Invoice {
    /// Encode invoice to bytes (simple format: id:amount)
    pub fn to_bytes(&self) -> Vec<u8> {
        format!("{}:{}", self.id, self.amount).into_bytes()
    }

    /// Parse from bytes. This is intentionally NOT public for direct use.
    /// Use ParseTransform instead to ensure provenance.
    fn from_bytes(bytes: &[u8]) -> Result<Self, TransformError> {
        let s = std::str::from_utf8(bytes)
            .map_err(|_| TransformError::C2pa("invalid UTF-8".into()))?;
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(TransformError::C2pa("invalid invoice format".into()));
        }
        let id = parts[0].parse()
            .map_err(|_| TransformError::C2pa("invalid id".into()))?;
        let amount = parts[1].parse()
            .map_err(|_| TransformError::C2pa("invalid amount".into()))?;
        Ok(Invoice { id, amount })
    }
}

impl C2paBindable for Invoice {
    fn content_hash(&self) -> ContentHash {
        ContentHash::compute(self.to_bytes())
    }

    fn media_type(&self) -> &str {
        "application/x-invoice"
    }
}

/// A simple grayscale image for Demo 2 (Redaction).
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

impl Image {
    /// Create a new image filled with a value.
    pub fn new(width: u32, height: u32, fill: u8) -> Self {
        Self {
            width,
            height,
            pixels: vec![fill; (width * height) as usize],
        }
    }

    /// Create a test pattern image.
    pub fn test_pattern(width: u32, height: u32) -> Self {
        let pixels: Vec<u8> = (0..(width * height))
            .map(|i| (i % 256) as u8)
            .collect();
        Self { width, height, pixels }
    }

    /// Get pixel at (x, y).
    pub fn get(&self, x: u32, y: u32) -> Option<u8> {
        if x < self.width && y < self.height {
            Some(self.pixels[(y * self.width + x) as usize])
        } else {
            None
        }
    }

    /// Set pixel at (x, y).
    pub fn set(&mut self, x: u32, y: u32, value: u8) {
        if x < self.width && y < self.height {
            self.pixels[(y * self.width + x) as usize] = value;
        }
    }
}

impl C2paBindable for Image {
    fn content_hash(&self) -> ContentHash {
        let mut data = Vec::new();
        data.extend_from_slice(&self.width.to_le_bytes());
        data.extend_from_slice(&self.height.to_le_bytes());
        data.extend_from_slice(&self.pixels);
        ContentHash::compute(data)
    }

    fn media_type(&self) -> &str {
        "image/x-grayscale"
    }
}

// ============================================================================
// Demo 1: ParseTransform - Verified Gate
// ============================================================================

/// Transform that parses verified bytes into a structured type.
///
/// # Type Safety
///
/// This transform ONLY accepts `C2pa<Vec<u8>, Verified>`.
/// Unverified bytes cannot be parsed - enforced at compile time.
///
/// ```compile_fail
/// use c2pa_primitives::*;
///
/// let unverified_bytes = C2pa::<Vec<u8>, Unverified>::new(
///     b"1:100".to_vec(),
///     Provenance::root("test", ClaimHash([0; 32]), AssetBinding::Hash(ContentHash([0; 32]))),
/// );
/// let parse = ParseTransform::<Invoice>::new();
/// let mut ctx = TransformContext::new("test");
/// // ERROR: expected Verified, found Unverified
/// let _ = parse.transform(&unverified_bytes, &mut ctx);
/// ```
pub struct ParseTransform<T> {
    _phantom: PhantomData<T>,
}

impl<T> ParseTransform<T> {
    pub fn new() -> Self {
        Self { _phantom: PhantomData }
    }
}

impl<T> Default for ParseTransform<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl C2paTransform<Vec<u8>, Invoice> for ParseTransform<Invoice> {
    fn transform(
        &self,
        input: &C2pa<Vec<u8>, Verified>,
        ctx: &mut TransformContext,
    ) -> Result<C2pa<Invoice, Verified>, TransformError> {
        let invoice = Invoice::from_bytes(input.payload())?;

        C2paBuilder::new(invoice)
            .generator(&ctx.generator)
            .add_ingredient(input, IngredientRelation::DerivedFrom)
            .sign(&TestSigner)
    }
}

// ============================================================================
// Demo 2: RedactTransform - Derivative with Provenance
// ============================================================================

/// Transform that redacts (masks) a rectangular region of an image.
///
/// The output image has provenance linking back to the original
/// with `derivedFrom` relationship.
pub struct RedactTransform {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl RedactTransform {
    pub fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }
}

impl C2paTransform<Image, Image> for RedactTransform {
    fn transform(
        &self,
        input: &C2pa<Image, Verified>,
        ctx: &mut TransformContext,
    ) -> Result<C2pa<Image, Verified>, TransformError> {
        let mut output = input.payload().clone();

        // Apply redaction (fill with 0)
        for dy in 0..self.h {
            for dx in 0..self.w {
                output.set(self.x + dx, self.y + dy, 0);
            }
        }

        C2paBuilder::new(output)
            .generator(&ctx.generator)
            .add_ingredient(input, IngredientRelation::DerivedFrom)
            .sign(&TestSigner)
    }
}

// ============================================================================
// Demo 3: CompositeTransform - Graph (DAG) Provenance
// ============================================================================

/// Trait for composing two verified sources into one output.
///
/// This creates a provenance DAG with multiple ingredients.
pub trait C2paComposite<A: C2paBindable, B: C2paBindable, O: C2paBindable> {
    fn compose(
        &self,
        a: &C2pa<A, Verified>,
        b: &C2pa<B, Verified>,
        ctx: &mut TransformContext,
    ) -> Result<C2pa<O, Verified>, TransformError>;
}

/// Composite transform that concatenates two images horizontally.
pub struct HConcatTransform;

impl C2paComposite<Image, Image, Image> for HConcatTransform {
    fn compose(
        &self,
        a: &C2pa<Image, Verified>,
        b: &C2pa<Image, Verified>,
        ctx: &mut TransformContext,
    ) -> Result<C2pa<Image, Verified>, TransformError> {
        let img_a = a.payload();
        let img_b = b.payload();

        // Heights must match for horizontal concat
        if img_a.height != img_b.height {
            return Err(TransformError::C2pa("height mismatch".into()));
        }

        let new_width = img_a.width + img_b.width;
        let height = img_a.height;
        let mut pixels = Vec::with_capacity((new_width * height) as usize);

        for y in 0..height {
            // Copy row from A
            let a_start = (y * img_a.width) as usize;
            let a_end = a_start + img_a.width as usize;
            pixels.extend_from_slice(&img_a.pixels[a_start..a_end]);

            // Copy row from B
            let b_start = (y * img_b.width) as usize;
            let b_end = b_start + img_b.width as usize;
            pixels.extend_from_slice(&img_b.pixels[b_start..b_end]);
        }

        let output = Image {
            width: new_width,
            height,
            pixels,
        };

        // Add BOTH sources as ingredients - this creates the DAG
        C2paBuilder::new(output)
            .generator(&ctx.generator)
            .add_ingredient(a, IngredientRelation::ComposedFrom)
            .add_ingredient(b, IngredientRelation::ComposedFrom)
            .sign(&TestSigner)
    }
}

/// Generic function-based composite transform.
pub struct FnComposite<F, A, B, O>
where
    F: Fn(&A, &B) -> O,
    A: C2paBindable,
    B: C2paBindable,
    O: C2paBindable,
{
    func: F,
    _phantom: PhantomData<(A, B, O)>,
}

impl<F, A, B, O> FnComposite<F, A, B, O>
where
    F: Fn(&A, &B) -> O,
    A: C2paBindable,
    B: C2paBindable,
    O: C2paBindable,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            _phantom: PhantomData,
        }
    }
}

impl<F, A, B, O> C2paComposite<A, B, O> for FnComposite<F, A, B, O>
where
    F: Fn(&A, &B) -> O,
    A: C2paBindable,
    B: C2paBindable,
    O: C2paBindable,
{
    fn compose(
        &self,
        a: &C2pa<A, Verified>,
        b: &C2pa<B, Verified>,
        ctx: &mut TransformContext,
    ) -> Result<C2pa<O, Verified>, TransformError> {
        let output = (self.func)(a.payload(), b.payload());

        C2paBuilder::new(output)
            .generator(&ctx.generator)
            .add_ingredient(a, IngredientRelation::ComposedFrom)
            .add_ingredient(b, IngredientRelation::ComposedFrom)
            .sign(&TestSigner)
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

// ============================================================================
// Transform Helper - API for macro-generated code
// ============================================================================

/// Helper module for macro-generated transform wrappers.
///
/// This module provides the building blocks used by `#[c2pa_transform]` macro.
/// It is public for macro expansion but considered internal API.
#[doc(hidden)]
pub mod transform_helper {
    use super::*;

    /// Build a transform result with provenance tracking.
    ///
    /// This function is used by the `#[c2pa_transform]` macro to construct
    /// the `C2pa<O, Verified>` result with proper provenance chain.
    ///
    /// # Arguments
    ///
    /// * `output` - The transformed payload
    /// * `input` - The verified input (becomes an ingredient)
    /// * `transform_name` - Name of the transform for provenance metadata
    /// * `relationship` - The ingredient relationship
    /// * `param_commits` - Parameter commits (name, hash) pairs - values not stored
    /// * `ctx` - Transform context
    pub fn build_transform_result<I, O>(
        output: O,
        input: &C2pa<I, Verified>,
        transform_name: &str,
        relationship: IngredientRelation,
        param_commits: Vec<(String, [u8; 32])>,
        ctx: &mut TransformContext,
    ) -> Result<C2pa<O, Verified>, TransformError>
    where
        I: C2paBindable,
        O: C2paBindable,
    {
        // Record transform metadata in context
        ctx.set_transform_name(transform_name);
        for (param_name, commit_hash) in &param_commits {
            ctx.add_param_commit(param_name.clone(), *commit_hash);
        }

        // Build the result with provenance
        let mut builder = C2paBuilder::new(output)
            .generator(&ctx.generator)
            .add_ingredient(input, relationship);

        // Add transform assertion if we have metadata
        if !transform_name.is_empty() || !param_commits.is_empty() {
            let assertion = build_transform_assertion(transform_name, &param_commits);
            builder = builder.add_assertion(assertion);
        }

        builder.sign(&TestSigner)
    }

    /// Build a custom assertion for transform metadata.
    fn build_transform_assertion(
        transform_name: &str,
        param_commits: &[(String, [u8; 32])],
    ) -> CustomAssertion {
        // Build a simple JSON-like structure for the assertion
        // Note: We only store commits (hashes), NOT raw parameter values
        let commits_json: String = param_commits
            .iter()
            .map(|(name, hash)| {
                format!(
                    r#""{}":{:?}"#,
                    name,
                    hex::encode(hash)
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        let json = format!(
            r#"{{"transform":"{}","param_commits":{{{}}}}}"#,
            transform_name,
            commits_json
        );

        CustomAssertion::json("c2pa.transform", &json)
    }
}

/// Simple hex encoding helper
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

// ============================================================================
// Thread-local Context API (for #[c2pa_pipeline])
// ============================================================================

use std::cell::RefCell;

thread_local! {
    static CURRENT_CTX: RefCell<Option<TransformContext>> = const { RefCell::new(None) };
}

/// Initialize a new pipeline context and run the closure within it.
///
/// Used by `#[c2pa_pipeline]` macro.
#[doc(hidden)]
pub fn with_new_ctx<F, R>(generator: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    CURRENT_CTX.with(|cell| {
        if cell.borrow().is_some() {
            panic!("c2pa_pipeline cannot be nested");
        }
        *cell.borrow_mut() = Some(TransformContext::new(generator));
    });

    let result = f();

    CURRENT_CTX.with(|cell| {
        *cell.borrow_mut() = None;
    });

    result
}

/// Execute a closure with mutable access to the current context.
///
/// Panics if called outside a `#[c2pa_pipeline]`.
#[doc(hidden)]
pub fn with_ctx<F, R>(f: F) -> R
where
    F: FnOnce(&mut TransformContext) -> R,
{
    CURRENT_CTX.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let ctx = borrow
            .as_mut()
            .expect("with_ctx called outside #[c2pa_pipeline]");
        f(ctx)
    })
}

/// Check if a pipeline context is currently active.
pub fn has_ctx() -> bool {
    CURRENT_CTX.with(|cell| cell.borrow().is_some())
}

// ============================================================================
// Debug Utilities - For demos and debugging
// ============================================================================

/// Debug utilities for inspecting C2PA provenance chains.
pub mod debug {
    use super::*;

    /// Format hash as short hex string (first 8 bytes).
    pub fn hash_short(hash: &[u8; 32]) -> String {
        hash.iter().take(8).map(|b| format!("{:02x}", b)).collect()
    }

    /// Print provenance info for a C2PA value.
    pub fn print_step<T>(label: &str, value: &C2pa<T, Verified>)
    where
        T: std::fmt::Debug + C2paBindable,
    {
        let prov = value.provenance();
        let content_hash = value.payload().content_hash();

        println!("\n┌─ {} ─────────────────────────────", label);
        println!("│ payload      : {:?}", value.payload());
        println!("│ manifest_id  : {}", prov.manifest_id);
        println!("│ claim_hash   : {}...", hash_short(prov.claim_hash.as_bytes()));
        println!("│ content_hash : {}...", hash_short(&content_hash.0));
        println!("│ ingredients  : {}", prov.ingredients.len());
        println!("└────────────────────────────────────");
    }

    /// Verify that ingredient's claim_hash matches parent's claim_hash.
    pub fn verify_chain<T, U>(child: &C2pa<T, Verified>, parent: &C2pa<U, Verified>, step_name: &str)
    where
        T: C2paBindable,
        U: C2paBindable,
    {
        let child_prov = child.provenance();
        let parent_prov = parent.provenance();

        if child_prov.ingredients.is_empty() {
            println!("  ⚠ {} has no ingredients to verify", step_name);
            return;
        }

        let ingredient_hash = &child_prov.ingredients[0].claim_hash;
        let parent_hash = &parent_prov.claim_hash;

        if ingredient_hash == parent_hash {
            println!(
                "  ✓ {} → parent claim_hash matches: {}...",
                step_name,
                hash_short(parent_hash.as_bytes())
            );
        } else {
            println!(
                "  ✗ {} → MISMATCH! ingredient: {}... vs parent: {}...",
                step_name,
                hash_short(ingredient_hash.as_bytes()),
                hash_short(parent_hash.as_bytes())
            );
        }
    }
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

    // Macro-generated transform tests are in tests/macro_tests.rs
    // (integration tests can use the crate as external dependency)
}
