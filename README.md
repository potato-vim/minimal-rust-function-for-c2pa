# C2PA Primitives

A minimal, educational, C2PA-inspired provenance DSL for Rust.

> This project does not implement the C2PA specification.
> It extracts the **essence** of what C2PA tries to solve — provenance, tamper-evidence, and accountability — and expresses it as a type-safe, minimal DSL.

---

## The Problem

In digital content and computation, a fundamental question persists:

**"Where did this come from, what happened to it, and why is it this way?"**

C2PA addresses this with cryptographic claims, provenance chains, and verification rules.
But the official spec and SDK are heavy, and the core insight can get lost in format details.

This project distills the **spirit** of C2PA into something you can read in an afternoon.

---

## Quick Demo

```rust
use c2pa_primitives::*;

#[c2pa_pipeline(generator = "demo")]
fn main() -> Result<(), TransformError> {
    let step1 = start_c2pa()?;           // source: 5
    let step2 = double_c2pa(&step1)?;    // transform: 10
    let step3 = add_ten_c2pa(&step2)?;   // transform: 20

    println!("Result: {}", step3.payload());
    Ok(())
}

#[c2pa_source]
fn start() -> u32 { 5 }

#[c2pa_transform(name = "double")]
fn double(x: &u32) -> u32 { x * 2 }

#[c2pa_transform(name = "add_ten")]
fn add_ten(x: &u32) -> u32 { x + 10 }
```

**Output:**

```
Result: 20
```

Behind the scenes, each step produces a `C2pa<T, Verified>` with:
- A cryptographic claim hash
- An ingredient reference to its parent's claim hash
- Content binding via hash

The chain `start -> double -> add_ten` is **cryptographically linked** — each step commits to its predecessor.

---

## What This Demo Shows

1. **Provenance is not an afterthought.**
   It is a first-class element of computation — not a log appended later.

2. **Verified values have structure.**
   A `C2pa<T, Verified>` can only come from:
   - A signed **source** (root of the chain)
   - A verified **transform** (preserves the chain)

3. **Transforms commit to their parents.**
   Each transformation references the parent's `claim_hash` as an ingredient.

4. **Results carry their history.**
   The output is not just a value — it is a value plus an explainable lineage.

---

## Core Ideas

### 1. Type-Safe Provenance

The Rust type system separates `Verified` from `Unverified` at compile time:

```rust
// This compiles:
fn process(input: &C2pa<u32, Verified>) -> C2pa<u32, Verified> { ... }

// This does NOT compile:
let unverified: C2pa<u32, Unverified> = ...;
process(&unverified);  // ERROR: expected Verified, found Unverified
```

Invalid data flow is rejected **before execution**.

### 2. Provenance as a Chain

Each transform:
- References its parent's `claim_hash` as an ingredient
- Produces a new `claim_hash` that commits to both the output and the parent

```
[source: claim_hash_1]
        |
        v  (ingredient: claim_hash_1)
[double: claim_hash_2]
        |
        v  (ingredient: claim_hash_2)
[add_ten: claim_hash_3]
```

This forms a hash chain — tamper-evident by construction.

### 3. DSL Ergonomics

Attribute macros keep computation code readable:

```rust
// You write this:
#[c2pa_transform(name = "double")]
fn double(x: &u32) -> u32 { x * 2 }

// The macro generates the provenance-aware wrapper:
// fn double_c2pa(&C2pa<u32, Verified>) -> Result<C2pa<u32, Verified>, _>
```

Provenance, context, and signing happen behind the scenes.

---

## System Structure

| Type | Role |
|------|------|
| `C2pa<T, Verified>` | A value with verified provenance |
| `C2pa<T, Unverified>` | A value awaiting verification |
| `Provenance` | Metadata: manifest ID, claim hash, ingredients |
| `ClaimHash` | SHA-256 commitment to the claim |
| `IngredientRef` | Reference to a parent's claim hash |
| `C2paBuilder` | Constructs verified values with signing |
| `TransformContext` | Pipeline state (generator label, assertions) |

### Attribute Macros

| Macro | Purpose |
|-------|---------|
| `#[c2pa_pipeline]` | Wraps a function with automatic context management |
| `#[c2pa_source]` | Defines a provenance origin (root of chain) |
| `#[c2pa_transform]` | Defines a provenance-preserving transformation |

---

## What This Is NOT

- **Not a C2PA specification implementation**
- **No JSON-LD, COSE, X.509, or media binding**
- **Not production-ready cryptography**

This project exists for **education, design exploration, and demonstrating the concept**.

If you need spec-compliant C2PA, use the official [c2pa-rs](https://github.com/contentauth/c2pa-rs) crate.

---

## Potential Applications

The pattern demonstrated here applies beyond media:

- **Data transformation pipelines** — track every ETL step
- **ML/AI inference provenance** — know which model, weights, and inputs produced an output
- **Compiler passes** — trace AST/IR transformations
- **Supply chain auditing** — cryptographic lineage for components
- **Reproducible computation** — verify that results came from claimed inputs and processes

This DSL shows **how** to embed provenance into computation — the specific domain is up to you.

---

## Running the Demo

```bash
cargo run -p c2pa_primitives
```

```bash
cargo test -p c2pa_primitives
```

---

## Closing

This project is not about implementing the C2PA spec.

It is about understanding **why** C2PA exists — and how provenance can become a first-class concept in computation.

---

## License

MIT OR Apache-2.0
