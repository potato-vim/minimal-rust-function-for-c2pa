# C2PA Primitives

任意のRust関数にC2PA来歴保証をつける最小実装。

## ストーリー

```rust
// 1. 任意の関数がある
fn double(x: &u32) -> u32 { x * 2 }

// 2. ラップする
let double = FnTransform::new(|x: &u32| x * 2, "double");

// 3. 来歴付きで動く
//    C2pa<u32, Verified> → C2pa<u32, Verified>
//    入力の来歴を継承し、署名された出力を返す
let result = double.transform(&verified_input, &mut ctx)?;
```

## Quick Start

```bash
cargo run    # 使用例の実行
cargo test   # テスト実行
```

## Files

| File | Role |
|------|------|
| `src/lib.rs` | 型・トレイト定義 |
| `src/main.rs` | 使用例 |

---

## System Structure

### 1. Core Type

```
C2pa<T, S>
├── payload: T       ... 関数を通したいコンテンツ
├── provenance       ... C2PA来歴メタデータ
└── PhantomData<S>   ... 検証フラグ（型レベル）

S = Unverified → 未検証
S = Verified   → 検証済（信頼経路からのみ生成可能）
```

### 2. Traits

| Trait | Role |
|-------|------|
| `C2paBindable` | ハッシュ計算可能な型 |
| `C2paTransform<I, O>` | 来歴を継承する変換 |
| `Signer` | 署名処理 |

### 3. Builder / Context

| Type | Role |
|------|------|
| `C2paBuilder<T>` | Verified値の構築 |
| `TransformContext` | 変換時のメタ情報 |

### 4. Data Structures

| Type | Role |
|------|------|
| `Provenance` | manifest_id, claim_hash, asset_binding, ingredients |
| `ClaimHash` | クレームのSHA-256ハッシュ |
| `ContentHash` | コンテンツのSHA-256ハッシュ |
| `AssetBinding` | Hash または Box(offset, length, hash) |
| `IngredientRef` | 親の参照情報 |
| `IngredientRelation` | ParentOf / ComponentOf / InputTo / DerivedFrom / ComposedFrom |

### 5. Demo Domain Types

| Type | Role |
|------|------|
| `Invoice` | Demo 1用: バイト列からパースされる構造体 |
| `Image` | Demo 2/3用: グレースケール画像 |
| `ParseTransform<T>` | `Vec<u8>` → `T` の来歴付きパース |
| `RedactTransform` | 画像の矩形領域をマスク（derivedFrom） |
| `HConcatTransform` | 2画像を横連結（composedFrom x 2） |
| `C2paComposite<A,B,O>` | 2入力合成トレイト（DAG生成） |

---

## Essential Demos

| Demo | 本質 |
|------|------|
| **Demo 1: Verified Gate** | 未検証バイトは`verify()`を通さないとパースできない。型でコンパイル時に強制。 |
| **Demo 2: Redaction** | 画像を加工しても`derivedFrom`として来歴が追跡される。改ざん隠蔽を防ぐ。 |
| **Demo 3: Graph** | 2つのVerified素材を合成すると`ingredients`が2本のDAGになる。チェーンではなくグラフ。 |

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
transform.transform(&unverified, &mut ctx);  // ERROR: expected Verified
```

---

## Extension Points

| 拡張 | 方法 |
|------|------|
| 新しい型の束縛 | `impl C2paBindable for YourType` |
| カスタム変換 | `impl C2paTransform<I, O> for YourTransform` |
| 実際の署名 | `impl Signer for YourSigner` (HSM, KMS等) |
