# C2PA Primitives

Rust関数に対するC2PA来歴追跡の最小実装。型システムで来歴保証を強制する。

## Quick Start

```bash
cargo run                              # デモ実行
cargo test                             # テスト実行
cargo run --example image_pipeline     # 画像パイプライン例
cargo run --example primitive_functions # 関数合成例
```

## Files

| File | Role |
|------|------|
| `src/lib.rs` | コアライブラリ。全ての型・トレイト定義 |
| `src/main.rs` | 対話的デモ |
| `examples/image_pipeline.rs` | 画像処理での使用例 |
| `examples/primitive_functions.rs` | 数値・文字列関数の合成例 |

---

## Interface Hierarchy

### Level 1: Core Type

```
┌─────────────────────────────────────────────────────────────┐
│  C2pa<T, S>                                                 │
│  ─────────────────────────────────────────────────────────  │
│  payload: T          値本体                                 │
│  provenance: Provenance  来歴メタデータ                     │
│  _state: PhantomData<S>  検証状態（型レベル）               │
└─────────────────────────────────────────────────────────────┘
         │
         ├── S = Unverified  → 未検証。自由に生成可能
         │
         └── S = Verified    → 検証済。生成経路が封印されている
```

**Why**: `Verified` マーカーを持つ値は信頼できる経路からのみ生成される。
コンパイル時に来歴要件を強制。

### Level 2: State Markers

```rust
pub struct Verified(());   // 内部コンストラクタ非公開 = 外部生成不可
pub struct Unverified;     // 自由に使用可能
```

**生成経路**:
| 経路 | 結果 |
|------|------|
| `C2pa::<T, Unverified>::new()` | 未検証値を生成 |
| `C2paBuilder::sign()` | `C2pa<T, Verified>` を生成 |
| `verify()` | `Unverified` → `Verified` に昇格 |
| `C2paTransform::transform()` | `Verified` → `Verified` 変換 |

---

### Level 3: Traits

#### `C2paBindable` - 束縛可能な型

```rust
pub trait C2paBindable {
    fn content_hash(&self) -> ContentHash;
    fn media_type(&self) -> &str { "application/octet-stream" }
}
```

**Role**: C2PAマニフェストに束縛できる型を定義。
**実装済**: `u8`〜`u128`, `i8`〜`i128`, `f32`, `f64`, `String`, `Vec<u8>`

**拡張**:
```rust
struct MyData { ... }

impl C2paBindable for MyData {
    fn content_hash(&self) -> ContentHash {
        ContentHash::compute(&self.serialize())
    }
}
```

#### `C2paTransform<I, O>` - 来歴保存変換

```rust
pub trait C2paTransform<I: C2paBindable, O: C2paBindable> {
    fn transform(
        &self,
        input: &C2pa<I, Verified>,  // 入力は必ず検証済
        ctx: &mut TransformContext,
    ) -> Result<C2pa<O, Verified>, TransformError>;  // 出力も検証済
}
```

**Role**: 変換が来歴チェーンを維持することを型で保証。
**制約**: 入力に `Verified` を要求 → 未検証データは変換不可。

**拡張**:
```rust
struct MyTransform;

impl C2paTransform<Image, Thumbnail> for MyTransform {
    fn transform(&self, input: &C2pa<Image, Verified>, ctx: &mut TransformContext)
        -> Result<C2pa<Thumbnail, Verified>, TransformError>
    {
        let thumb = create_thumbnail(input.payload());

        C2paBuilder::new(thumb)
            .add_ingredient(input, IngredientRelation::ParentOf)
            .sign(&signer)
    }
}
```

#### `Signer` - 署名インターフェース

```rust
pub trait Signer {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>, TransformError>;
    fn certificate_chain(&self) -> &[Vec<u8>];
}
```

**Role**: 署名処理の抽象化。
**提供**: `TestSigner` (プロトタイプ用ダミー)

---

### Level 4: Builder & Context

#### `C2paBuilder<T>`

```rust
C2paBuilder::new(payload)
    .generator("MyApp/1.0")
    .add_ingredient(&parent, IngredientRelation::ParentOf)
    .sign(&signer)?
```

| Method | Role |
|--------|------|
| `new(T)` | ペイロードで初期化 |
| `generator(str)` | 生成者ラベル設定 |
| `add_ingredient(&C2pa<I, Verified>, Relation)` | 親参照を追加 |
| `sign(&Signer)` | 署名して `C2pa<T, Verified>` を返す |

#### `TransformContext`

```rust
TransformContext::new("MyPipeline/1.0")
    .with_timestamp(true)
    .add_assertion(CustomAssertion::json("label", "{}"))
```

| Field | Role |
|-------|------|
| `generator` | 変換パイプラインの識別子 |
| `require_timestamp` | タイムスタンプ要求フラグ |
| `assertions` | カスタムアサーション |

---

### Level 5: Data Structures

#### `Provenance`

```rust
pub struct Provenance {
    pub manifest_id: String,        // JUMBF URI
    pub claim_hash: ClaimHash,      // SHA-256
    pub asset_binding: AssetBinding,
    pub ingredients: Vec<IngredientRef>,
}
```

#### `IngredientRef`

```rust
pub struct IngredientRef {
    pub claim_hash: ClaimHash,
    pub asset_binding: AssetBinding,
    pub relationship: IngredientRelation,  // ParentOf | ComponentOf | InputTo
}
```

#### `AssetBinding`

```rust
pub enum AssetBinding {
    Hash(ContentHash),
    Box { offset: u64, length: u64, hash: ContentHash },
}
```

---

### Level 6: Convenience

#### `FnTransform<F, I, O>`

関数をラップして `C2paTransform` に変換:

```rust
let double = FnTransform::new(|x: &u32| x * 2, "double");
let result = double.transform(&verified_input, &mut ctx)?;
```

#### `verify()`

未検証値を検証済に昇格:

```rust
let verified = verify(unverified, &expected_claim_hash)?;
```

---

## Type Safety Diagram

```
                    ┌──────────────────┐
                    │   External       │
                    │   Trusted Source │
                    └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │ C2paBuilder      │
                    │   .sign()        │
                    └────────┬─────────┘
                             │
         ┌───────────────────▼───────────────────┐
         │       C2pa<T, Verified>               │
         │  (Only constructible via trusted path)│
         └───────────────────┬───────────────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
         .payload()    .transform()    verify()
              │              │              │
              ▼              ▼              ▼
           &T          C2pa<O, Verified>   C2pa<T, Verified>
                        (with parent ref)   (from Unverified)

         ┌───────────────────────────────────────┐
         │       C2pa<T, Unverified>             │
         │  (Free to create, cannot use where    │
         │   Verified is required)               │
         └───────────────────────────────────────┘
```

---

## What Won't Compile

```rust
// 1. Unverified を Verified として使用
fn needs_verified(v: &C2pa<u32, Verified>) { ... }
let unverified = C2pa::<u32, Unverified>::new(...);
needs_verified(&unverified);  // ERROR: expected Verified

// 2. Verified の直接構築
let fake = C2pa::<u32, Verified>::new(...);  // ERROR: private constructor

// 3. 未検証入力の変換
let transform = FnTransform::new(...);
transform.transform(&unverified, &mut ctx);  // ERROR: expected Verified
```

---

## Extension Points

| 拡張 | 方法 |
|------|------|
| 新しい型の束縛 | `impl C2paBindable for YourType` |
| カスタム変換 | `impl C2paTransform<I, O> for YourTransform` |
| 実際の署名 | `impl Signer for YourSigner` (HSM, KMS等) |
| c2paクレート統合 | `Cargo.toml` の `c2pa-integration` feature有効化 |

---

## Design Philosophy

1. **型で保証**: ランタイムチェックではなくコンパイル時に来歴要件を強制
2. **最小インターフェース**: 必要最小限のトレイトとメソッド
3. **合成可能**: `transform` は任意に連鎖可能、各ステップが来歴を継承
4. **内部実装は具体を見ろ**: このREADMEはインターフェースに集中、実装詳細は `src/lib.rs`
# minimal-rust-function-for-c2pa
