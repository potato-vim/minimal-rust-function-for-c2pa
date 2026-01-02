//! This should fail to compile because `double_c2pa` only accepts Verified input.

use c2pa_primitives::*;

#[c2pa_transform(name = "double")]
fn double(x: &u32) -> u32 {
    x * 2
}

fn main() {
    // Create an unverified value
    let unverified: C2pa<u32, Unverified> = C2pa::new(
        42,
        Provenance::root(
            "test",
            ClaimHash([0; 32]),
            AssetBinding::Hash(ContentHash([0; 32])),
        ),
    );

    // This should fail: double_c2pa expects &C2pa<u32, Verified>
    let _ = double_c2pa(&unverified);
}
