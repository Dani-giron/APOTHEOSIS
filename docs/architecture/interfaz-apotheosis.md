# Interface Document: APOTHEOSIS 2

## Table of Contents

1. [Interface Identity](#1-interface-identity)
2. [Provided Resources](#2-provided-resources)
   - 2.1 `Apotheosis` Facade
   - 2.2 Contract Traits
   - 2.3 Ready-to-Use Types
3. [Error Handling](#3-error-handling)
4. [Variability](#4-variability)
5. [Quality Attributes](#5-quality-attributes)
6. [Design Rationale](#6-design-rationale)
7. [Usage Guide](#7-usage-guide)

---

## 1. Interface Identity

The crate is named `apotheosis2`. It is a Rust library for approximate nearest-neighbor (ANN) search over fuzzy hashes, primarily TLSH. The element documented here is the crate as a whole, not a running component or network service.

The interface of `apotheosis2` is exposed exclusively through the Rust API: the set of public types, traits, and functions reachable from `src/lib.rs` via three root modules, `controllers`, `datalayer`, and `export`. There is no CLI, REST, or RPC interface; the only way to interact with the library is as a dependency in a Rust project.

The crate version, Rust edition, minimum supported Rust version, and license are declared in `Cargo.toml` and are not restated here. Internal organization is in the module-view document; runtime topology in the C&C view.

#### Environmental assumptions

`apotheosis2` is not published on crates.io. Client projects must declare it via a local path or git URL. The crate depends on `tlsh2` from a private git fork (`https://github.com/danielhuici/tlsh`); this fork must be reachable at build time and must be declared explicitly in the client's `Cargo.toml` (or resolved via `[patch]`). `serde_json` and `tlsh2` types appear in the public API but are not re-exported; the client must add them as direct dependencies when needed.

---

## 2. Provided Resources

Three categories: the `Apotheosis` facade (main entry point), the contract traits (`ApotheosisRecord`, `RadixKeyMapping`, `DistanceAlgorithm`) that connect client domain types to the search engine, and ready-to-use concrete implementations of those traits.

### 2.1 `Apotheosis` Facade

`Apotheosis` is the single entry point most clients need. It encapsulates three internal structures that it keeps in sync: the HNSW graph for approximate search, the radix tree for O(1) exact-match lookup, and the record vector for retrieving original data. These three fields are private; the only way in is through the methods below. Small read accessors — `len()`, `is_empty()`, `draw_model()` (structural view of the graph layers), and `record(index)` — expose counts and read-only views without breaking encapsulation; the `export::gexf` module and the test suite consume the facade through them. The generic parameters (`R`, `D`, `M`, `M0`, `EF`, `HEURISTIC`) are documented in §4.

```rust
pub struct Apotheosis<
    R,
    D,
    const M: usize = 16,
    const M0: usize = 32,
    const EF: usize = 400,
    const HEURISTIC: bool = false,
>
where
    R: ApotheosisRecord,
    D: DistanceAlgorithm<R::MetricId> + Default,
```

The three fields it holds are private and do not appear in the signature above. The type derives `Serialize` and `Deserialize` with explicit serde bounds, which is what makes `dump` and `load` available when `R`, `D`, and `R::MetricId` are themselves serializable.

#### 2.1.1 `new`

**Syntax**

```rust
pub fn new() -> Self
```

**Semantics**

Constructs an empty `Apotheosis` instance. The HNSW graph, the radix tree, and the record vector are all initialized but empty. No arguments are required: the type and const parameters (`R`, `D`, `M`, `M0`, `EF`) are resolved by type inference or annotation at the call site.

**Error Handling**

No explicit error paths. Does not return `Result`. No panics are identifiable in the code of this method.

---

#### 2.1.2 `insert`

**Syntax**

```rust
pub fn insert(&mut self, item: R) -> bool
```

**Semantics**

Inserts a record into all three internal structures: the `search_id()` of the record is added to the HNSW graph; if the `MetricId` type implements `to_radix_key()` returning `Some(...)`, the resulting key is also indexed in the radix tree; and the full record is appended to the `records` vector.

The meaning of the return value depends on whether the `MetricId` type activates the radix fast-path:

- If `item.search_id().to_radix_key()` returns `Some(key)` (fast-path active): the method checks whether `key` already exists in the radix tree. If it does, a warning is emitted and the method returns `false`. If it does not, the record is inserted into the radix tree and `records` and the method returns `true`.
- If `item.search_id().to_radix_key()` returns `None` (fast-path inactive): no duplicate check is performed. The method always inserts into `records` and always returns `true`. `true` in this path means the insertion was attempted, not that the item is unique in the index.

**Error Handling**

Does not return `Result`. No panics are identifiable in the code of this method.

**Notes**

The duplicate warning is informational only and is not propagated as an error to the caller. Additionally, `search_id()` is called twice on the same `item` within the method (once to derive the radix key, once for HNSW insertion): if an implementation of `search_id()` had side effects, the result could be unexpected.


---

#### 2.1.3 `search`

**Syntax**

```rust
pub fn search(
    &self,
    query: &R::MetricId,
    k: usize,
    ef_search: Option<usize>,
) -> Vec<(u32, &R)>
```

**Semantics**

Performs a search for the `k` nearest neighbors of the given `query`. Each element of the result vector is a tuple `(distance, record_reference)`: the `u32` is the distance computed by algorithm `D` between the `query` and the found record (the concrete units depend on the `D` implementation; for `TlshDistance` it is the TLSH distance, for `NormalDistance` over `u32` it is the absolute difference); the `&R` is a reference to the full record stored in `self.records`.

The `ef_search` parameter controls the size of the dynamic candidate list during HNSW graph traversal. If `None` is passed, the effective value is `24`. Higher values improve result quality at the cost of increased search time.

The method follows two internal paths depending on the type of `query`:

- If `query.to_radix_key()` returns `Some(key)` and that key exists in the radix tree with an associated index (exact match): graph traversal is skipped and the neighbors of the matched node in HNSW layer 0 are retrieved directly, including the matched node itself with distance `0`.
- In all other cases: the standard k-NN search over the HNSW graph is executed.

Regardless of which path was taken, the results are sorted by ascending distance and truncated to `k` before being returned. If the index (or, on the fast-path, the matched node's neighbor list) contains fewer than `k` elements, the result vector will have fewer than `k` elements.

**Error Handling**

Does not return `Result`. No panics are identifiable in the normal flow.

**Notes**

The `ef_search` parameter is independent of the type-level constant `EF`. `EF` controls the exploration factor during graph construction; `ef_search` controls the search. The default applied when `None` is passed is considerably smaller than the default `EF` the graph was built with, so passing `None` in settings that require high recall may produce suboptimal results.


---

#### 2.1.4 `dump`

**Syntax**

```rust
pub fn dump<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>>
where
    Self: serde::Serialize,
```

**Semantics**

Serializes the full instance to disk at the path indicated by `path`. The generated file has the following binary format:

- Bytes 0-3: magic bytes `APOT` in ASCII.
- Bytes 4-7: value of `M` encoded as `u32` in little-endian.
- Bytes 8-11: value of `M0` encoded as `u32` in little-endian.
- Bytes 12-15: value of `EF` encoded as `u32` in little-endian.
- Byte 16: value of `HEURISTIC` as a single byte, 0 or 1.
- Byte 17: length in bytes of the distance type name, as a single byte.
- Next N bytes: the distance type name in UTF-8 (e.g. `TlshDistance`).
- Remaining bytes: body serialized with `bincode`.

The format does not include an independent version field: internal changes to `Hnsw` or `RadixNode` between crate versions may produce silently incompatible files.

**Error Handling**

Returns `Err(Box<dyn std::error::Error>)` on I/O error when creating or writing the file, or on a `bincode` serialization error.

**Notes**

The bound `where Self: serde::Serialize` implies that `R`, `D`, and `R::MetricId` must implement `serde::Serialize`. Which provided record types satisfy this bound is documented in §2.3.

---

#### 2.1.5 `load`

**Syntax**

```rust
pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>>
where
    Self: serde::de::DeserializeOwned,
```

**Semantics**

Associated function (does not take `self`). Reads a file produced by `dump` and reconstructs an `Apotheosis` instance. Reads the header first: verifies the `APOT` magic, checks that the values of `M`, `M0`, `EF`, and `HEURISTIC` in the file match the const parameters of the type on which `load` is invoked, and checks that the distance type name recorded in the file matches the `D` the loading type was compiled with. If validation passes, deserializes the body with `bincode`.

**Error Handling**

Returns `Err(Box<dyn std::error::Error>)` in the following cases:

- I/O error when opening the file or reading the header.
- Incorrect magic bytes: message `"Invalid Apotheosis model file (missing magic bytes)"`.
- Parameter mismatch: a message naming both the stored and the expected values of `M`, `M0`, `EF`, and `HEURISTIC`.
- Distance type mismatch: a message naming both the distance the file was built with and the one it is being loaded as.
- `bincode` deserialization error.

**Notes**

The bound `where Self: serde::de::DeserializeOwned` imposes the same restrictions as the `dump` bound on `serde::Serialize`. Validation is strict and rejects a mismatch on any of the four graph parameters and on the distance type name, but it does not cover the record type `R`. On load, the internal HNSW `prng` field is re-initialized with seed `42`; this does not affect search results, only the random insertion level for future `insert` calls.

---

#### 2.1.6 `export::gexf::draw`

**Syntax**

```rust
// free function in the export::gexf module, not a facade method
pub fn draw<R, D, /* const params */, P: AsRef<Path>>(model: &Apotheosis<...>, path: P)
```

**Semantics**

GEXF export lives in the `export::gexf` module and consumes the facade through its public read API (`draw_model()`, `record()`). Exports the HNSW graph structure to GEXF files for visualization in external tools such as Gephi. Generates one file per graph layer (layer 0 plus all existing upper layers). The file name pattern is `{stem}_layer{N}.gexf`, where `{stem}` is the base name of `path` without extension and `N` is the layer index starting at `0`. If `path` has no recognizable base name, the pattern is `layer{N}.gexf`.

Each GEXF file contains nodes (one per entry in that HNSW layer, identified by its feature index), undirected edges weighted by the distance between connected nodes, and node attributes obtained from `R::get_attributes()`. The attribute schema is declared by the `gexf` dependency itself, which auto-collects the node attribute keys.

**Error Handling**

Does not return `Result`. I/O errors when writing each GEXF file are silently discarded: the code uses `let _ = save_gexf(...)`. GEXF XML serialization errors (`gexf.to_string().unwrap()`) produce a panic.



**Preconditions / Postconditions**

If the model is empty, the files are generated regardless, but without nodes or attribute schema.

### 2.2 Contract Traits

#### 2.2.1 `ApotheosisRecord`

**Syntax**

```rust
pub trait ApotheosisRecord {
    type MetricId: Clone + RadixKeyMapping;
    fn search_id(&self) -> Self::MetricId;

    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![]
    }
}
```

**Semantics**

`ApotheosisRecord` represents any entity that can be inserted into the index. The associated type `MetricId` is the metric space in which the HNSW operates: it must implement `Clone` so the library can store a copy of the identifier separately from the full record, and `RadixKeyMapping` to optionally enable the exact-match fast-path. The method `search_id()` extracts from the record the metric identifier that will be used both in HNSW distance computations and as radix tree keys. `get_attributes()` returns key-value metadata pairs for GEXF export; its default implementation returns an empty list.

**Notes**

`search_id()` is called in the hot paths of `insert` (twice per insertion) and during internal graph initialization; its implementation should be cheap. `get_attributes()` is only relevant when `draw()` is called; a client that does not use GEXF export can ignore it. For provided implementations, see §2.3.

---

#### 2.2.2 `RadixKeyMapping`

**Syntax**

```rust
pub trait RadixKeyMapping {
    fn to_radix_key(&self) -> Option<Vec<u8>> {
        None
    }
}
```

**Semantics**

`RadixKeyMapping` defines whether a `MetricId` type has a canonical byte-sequence representation for O(1) exact-match lookup in the radix tree. `to_radix_key()` returns `Some(bytes)` to enable the fast-path or `None` to disable it. Accepting the default `None` (overriding nothing) is the minimal valid implementation; it silently disables the fast-path. The fast-path is only worthwhile when the metric space has a canonical, unambiguous byte representation.

**Implementations Provided by the Library**

- `impl RadixKeyMapping for TlshDefault`: returns `Some(self.hash().to_vec())`, the raw bytes of the TLSH hash.
- `impl RadixKeyMapping for u32`: returns `Some(self.to_string().into_bytes())`, the decimal representation of the integer as UTF-8.

**Notes**

`TlshDefault` belongs to the `tlsh2` crate, which is not re-exported by `apotheosis2`. A client that needs to construct or manipulate `TlshDefault` values directly must add `tlsh2` as a direct dependency.

The `u32` implementation encodes the integer as decimal text, not as four bytes in numeric byte order. The key is valid and produces no collisions but occupies more bytes than necessary.

---

#### 2.2.3 `DistanceAlgorithm`

**Syntax**

```rust
pub trait DistanceAlgorithm<ID> {
    fn calculate_distance(&self, a: &ID, b: &ID) -> u32;
}
```

**Semantics**

`DistanceAlgorithm<ID>` defines a distance or dissimilarity function between two values of metric space `ID`. `calculate_distance` returns a `u32`: smaller values indicate greater proximity. The HNSW does not assume metric axioms; it only requires that smaller values correspond to more similar pairs.

**Implementations Provided by the Library**

- `NormalDistance` for `u32`: computes `a.abs_diff(*b)`.
- `TlshDistance` for `TlshDefault`: computes `a.diff(&b, true) as u32` (TLSH distance, `len_diff=true` hardcoded).

**Notes**

The bound `D: Default` is not in the definition of the `DistanceAlgorithm` trait but in the where-clause of `Apotheosis` and `Hnsw`. Any custom implementation must also implement `Default`. If the algorithm requires runtime configuration, carry it as fields and implement `Default` with the desired defaults.

When using `dump`/`load`, `D` must additionally implement `serde::Serialize` and `serde::Deserialize`. Both provided implementations do so via `#[derive]`.

### 2.3 Ready-to-Use Types

#### Record types

#### `SimpleRecord<ID>`

**Definition**

```rust
pub struct SimpleRecord<ID> {
    pub id: ID,
    pub radix_key: String,
}
```

**Implements**

`ApotheosisRecord` (when `ID: Clone + RadixKeyMapping`), `Clone`, `serde::Serialize`, `serde::Deserialize`.

**Constructor**

No public constructor. Constructed directly by field: `SimpleRecord { id, radix_key }`.

**Notes**

The `radix_key: String` field is stored but is not read by the library; the actual radix key is derived at runtime via `id.to_radix_key()`. Compatible with `dump`/`load` as long as `ID` implements `Serialize` and `Deserialize`.

---

#### `SimpleNumberRecord`

**Definition**

Type alias: `pub type SimpleNumberRecord = SimpleRecord<u32>;`

**Implements**

Same as `SimpleRecord<u32>`: `ApotheosisRecord`, `Clone`, `serde::Serialize`, `serde::Deserialize`.

**Constructor**

```rust
pub fn create(s: String) -> Result<Self, Box<dyn std::error::Error>>
```

**Notes**

`create` returns `Err` if `s` is not parseable as `u32`, with the offending input in the message. Compatible with `dump`/`load`.

---

#### `SimpleTlshRecord`

**Definition**

Type alias: `pub type SimpleTlshRecord = SimpleRecord<TlshDefault>;`

**Implements**

Same as `SimpleRecord<TlshDefault>`: `ApotheosisRecord`, `Clone`, `serde::Serialize`, `serde::Deserialize`.

**Constructor**

```rust
pub fn create(s: String) -> Result<Self, Box<dyn std::error::Error>>
```

**Notes**

`create` returns `Err` if `s` is not a valid TLSH hash string. The client must add `tlsh2` as a direct dependency to construct `TlshDefault` values for queries. Compatible with `dump`/`load`.

---

#### `GenericJsonRecord`

**Definition**

```rust
pub struct GenericJsonRecord {
    pub id: TlshDefault,
    pub radix_key: String,
    pub metadata: serde_json::Value,
}
```

**Implements**

`ApotheosisRecord`, `Clone`, `serde::Serialize`, `serde::Deserialize`.

**Constructor**

```rust
pub fn create(s: String, metadata: serde_json::Value) -> Result<Self, Box<dyn std::error::Error>>
```

**Notes**

`create` returns `Err` if `s` is not a valid TLSH hash string. `get_attributes()` serializes `metadata` as flat key-value pairs: if `metadata` is a JSON object, one entry per top-level field; otherwise a single entry with key `"metadata"`. The `metadata` field is `serde_json::Value`, which is not re-exported by `apotheosis2`; the client must add `serde_json` as a direct dependency. Compatible with `dump`/`load`.

---

#### `WinModuleRecord`

**Definition**

```rust
pub struct WinModuleRecord {
    pub hash: TlshDefault,
    pub id: u32,
    pub file_version: String,
    pub internal_filename: String,
    pub product: String,
    pub company: String,
    pub os_id: u32,
}
```

**Implements**

`ApotheosisRecord`, `Clone`. Does NOT implement `serde::Serialize` or `serde::Deserialize`.

**Constructor**

No public constructor. Constructed directly by field.

**Notes**

Cannot be used with `dump`/`load`. `get_attributes()` exposes six fields: `id`, `file_version`, `internal_filename`, `product`, `company`, `os_id`.

---

#### `BinaryRecord`

**Definition**

```rust
pub struct BinaryRecord {
    pub hash: TlshDefault,
    pub versions_names: Vec<(String, String)>,
}
```

**Implements**

`ApotheosisRecord`. Does NOT implement `Clone`, `serde::Serialize`, or `serde::Deserialize`.

**Constructor**

No public constructor. Constructed directly by field.

**Notes**

Cannot be used with `dump`/`load`. The absence of `Clone` prevents use in contexts that require cloning the record. `get_attributes()` returns a single entry `("version_names", "")` with an empty value in the current implementation.

---

#### `FileRecord`

**Definition**

```rust
pub struct FileRecord {
    pub hash: TlshDefault,
    pub filename: String,
    pub file_path: String,
    pub version: String,
    pub size: u64,
    pub timestamp: u64,
}
```

**Implements**

None of the contract traits. Does NOT implement `ApotheosisRecord`, `Clone`, `serde::Serialize`, or `serde::Deserialize`.

**Constructor**

No public constructor.

**Notes**

Cannot be used as type `R` in `Apotheosis<R, D, ...>`. Its presence as a public type without any contract implementation has no current utility for a client.


---

#### Distance types

#### `NormalDistance`

**Definition**

```rust
pub struct NormalDistance;
```

**Implements**

`DistanceAlgorithm<u32>`, `Default`, `serde::Serialize`, `serde::Deserialize`.

**Distance semantics**

Computes `a.abs_diff(*b)`: the absolute difference between two unsigned 32-bit integers.

**Notes**

Compatible with `dump`/`load`. Typical use: `Apotheosis<SimpleNumberRecord, NormalDistance>`.

---

#### `TlshDistance`

**Definition**

```rust
pub struct TlshDistance;
```

**Implements**

`DistanceAlgorithm<TlshDefault>`, `Default`, `Clone`, `serde::Serialize`, `serde::Deserialize`.

**Distance semantics**

Computes `a.diff(&b, true) as u32`: the TLSH distance with `len_diff=true` (length difference included), hardcoded.

**Notes**

The `len_diff=true` parameter is not configurable. Compatible with `dump`/`load`. Typical use: `Apotheosis<SimpleTlshRecord, TlshDistance>` or `Apotheosis<GenericJsonRecord, TlshDistance>`.

---

## 3. Error Handling

The crate does not adopt a uniform error-handling strategy: different methods use different mechanisms, and some failure conditions are not surfaced to the caller at all.

#### `Result<_, Box<dyn std::error::Error>>`

Used by `dump`, `load`, and the `create` record constructors. The error type is a dynamic trait object that can wrap any I/O or serialization error. The caller must match on or propagate the error with `?`. There are no structured error types that would allow programmatic distinction between the possible causes (I/O failure, parameter mismatch, malformed file).

#### Panics

`export::gexf::draw` can panic if the internal GEXF XML serialization fails (`gexf.to_string().unwrap()` in `save_gexf`). No public facade method has identifiable panics in the normal flow; the `create` record constructors return `Err` on malformed input instead of panicking.

#### Absence of error return

`insert` returns `bool` rather than `Result`: duplicate keys are signaled with `false` and a warning, without propagating an error to the caller. `export::gexf::draw` has no explicit error paths: I/O errors when writing each GEXF file are silently discarded via `let _ = save_gexf(...)`.

---

## 4. Variability

All variation points are resolved at compile time through Rust's type system (const-generics and traits), with the exception of `ef_search`, which is the only tuning parameter that takes effect at runtime.

#### `R`: Record type

Type parameter. Must implement `ApotheosisRecord`. Defines the data domain that the index stores. The client can use the provided types (§2.3) or implement the trait on a custom type.

#### `D`: Distance algorithm

Type parameter. Must implement `DistanceAlgorithm<R::MetricId>` and `Default`. Determines the similarity function for the metric space. The client can use `NormalDistance` or `TlshDistance`, or implement the trait on a custom type. To use `dump`/`load`, `D` must additionally implement `serde::Serialize` and `serde::Deserialize`.

#### `M`: Upper-layer graph connectivity

Const-generic parameter. Default value: `16`. Maximum number of neighbors per node in the upper layers of the HNSW graph. Higher values improve search quality at the cost of greater memory usage and longer construction time. Resolved at compile time: changing `M` requires recompilation, and `load` rejects files built with a different value.

#### `M0`: Base-layer graph connectivity

Const-generic parameter. Default value: `32`. Maximum number of neighbors per node in layer 0 (the layer containing all elements). Conventionally twice `M`. Same compile-time and compatibility implications as `M`.

#### `EF`: Construction exploration factor

Const-generic parameter. Default value: `400`. Controls the size of the dynamic candidate list during insertion. Higher values produce a higher-quality graph at the cost of longer construction time. Does not affect search quality directly (see `ef_search`). Same compile-time and compatibility implications as `M` and `M0`.

#### `ef_search`: Search exploration factor (runtime)

Parameter of `search()`, type `Option<usize>`. The only runtime variation point. When `None` is passed, the effective value is `24`. Controls the quality-versus-speed trade-off for each individual search call. Does not persist between calls.

---

## 5. Quality Attributes

*For a full quality-attribute analysis see [`quality-attributes.md`](quality-attributes.md).*

#### Performance: sublinear search complexity

`search` operates in sublinear complexity in practice thanks to the HNSW graph. The exact complexity depends on `M`, `M0`, `EF`, and `ef_search`. When the radix fast-path finds an exact match, the search is O(key length) in the radix tree plus O(neighbors in layer 0).

#### Reproducibility: model persistence

The `dump`/`load` pair guarantees that a model built in one process can be recovered in another producing identical search results, with the exception of the PRNG state (re-initialized with seed `42` on `load`, affecting only the random insertion level for future `insert` calls). The strict validation of `M`/`M0`/`EF` in `load` guarantees that a model with incompatible parameters will not be silently loaded.

#### Sync invariant integrity

The three internal structures (`hnsw`, `radix`, `records`) stay in sync through a shared index `i`, maintained entirely by `insert()`. Because the fields are private, there is no way to reach in from outside and break that invariant; the compiler enforces it, not just convention.

#### Thread safety

`Apotheosis` auto-implements `Send + Sync` when all type parameters (`R`, `D`, `R::MetricId`) are `Send + Sync`. No interior mutability is used in any component. Concurrent reads via `search` are safe; mutation via `insert` requires exclusive access (`&mut self`).



---

## 6. Design Rationale

### 6.1 Design decisions that shape the contract

#### Metric injection via trait

`DistanceAlgorithm<ID>` is a type parameter resolved at compile time, not a function pointer or a dynamic trait object. Different metric spaces become distinct `Apotheosis` types, mutually incompatible at the compiler level.

#### Radix fast-path as silent opt-in

Enabling the radix fast-path requires no change to the calls to `search` or `insert`: it depends solely on whether `to_radix_key()` returns `Some` or `None`. The fast-path can be added or removed by changing only the trait implementation, without touching client call sites.

#### Const-generics for graph parameters

`M`, `M0`, and `EF` are const-generics rather than runtime configuration fields. Different graph configurations become distinct, mutually incompatible types at the compiler level, preventing accidental mixing of models with different parameters.

### 6.2 Encapsulation

The fields `hnsw`, `radix`, and `records` of `Apotheosis` are private. External callers only ever see them through the public methods documented above, which is what makes the guarantees in §5 actually hold. The types `Hnsw`, `RadixNode`, and `HnswNode` themselves are documented in the module-view document, for readers who need to understand what's inside.


---

## 7. Usage Guide

Examples assume `apotheosis2` and its direct dependencies are declared in `Cargo.toml`.

#### Cargo.toml

```toml
[dependencies]
apotheosis2 = { git = "https://github.com/reverseame/APOTHEOSIS", branch = "apotheosis2" }
tlsh2 = { git = "https://github.com/danielhuici/tlsh", features = ["diff", "serde"] }
serde_json = "*"  # only if using GenericJsonRecord or serde_json::Value directly
```

Use a `path` dependency instead of the git one when working against a local checkout. Pin versions as your project requires; the snippet above is deliberately unpinned.

#### Example 1: Search with provided types (TLSH)

```rust
use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::TlshDistance;
use apotheosis2::datalayer::record::SimpleTlshRecord;
// TlshDefault belongs to the `tlsh2` crate; add it as a direct dependency in Cargo.toml.
use std::str::FromStr;
use tlsh2::TlshDefault;

fn main() {
    let mut index: Apotheosis<SimpleTlshRecord, TlshDistance> = Apotheosis::new();

    // SimpleTlshRecord::create returns Err on invalid TLSH strings.
    // Replace with real TLSH hash strings produced by your data pipeline.
    index.insert(SimpleTlshRecord::create("<tlsh_hash_a>".to_string()).expect("invalid TLSH hash"));
    index.insert(SimpleTlshRecord::create("<tlsh_hash_b>".to_string()).expect("invalid TLSH hash"));

    let query = TlshDefault::from_str("<tlsh_hash_a>").expect("invalid TLSH hash");

    // Passing None for ef_search applies the library default; see section 4.
    let results: Vec<(u32, &SimpleTlshRecord)> = index.search(&query, 5, None);

    for (distance, record) in &results {
        println!("distance={} radix_key={}", distance, record.radix_key);
    }
}
```

#### Example 2: Custom record type

```rust
use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::TlshDistance;
use apotheosis2::datalayer::record::ApotheosisRecord;
// TlshDefault belongs to the `tlsh2` crate; add it as a direct dependency in Cargo.toml.
use std::str::FromStr;
use tlsh2::TlshDefault;

// Derive serde traits only if dump/load is needed; Clone is not required by Apotheosis itself.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct MyRecord {
    hash: TlshDefault,
    label: String,
}

impl ApotheosisRecord for MyRecord {
    type MetricId = TlshDefault;

    fn search_id(&self) -> TlshDefault {
        self.hash.clone()
    }

    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![("label".to_string(), self.label.clone())]
    }
}

fn main() {
    let mut index: Apotheosis<MyRecord, TlshDistance> = Apotheosis::new();

    // Replace with a real TLSH hash string; this placeholder will fail to parse.
    let record = MyRecord {
        hash: TlshDefault::from_str("<tlsh_hash_a>").expect("invalid TLSH hash"),
        label: "example".to_string(),
    };
    index.insert(record);

    let query = TlshDefault::from_str("<tlsh_hash_a>").expect("invalid TLSH hash");
    let results = index.search(&query, 3, Some(100));
    for (distance, rec) in &results {
        println!("distance={} label={}", distance, rec.label);
    }
}
```

#### Example 3: Persistence with dump/load

```rust
use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::NormalDistance;
use apotheosis2::datalayer::record::SimpleNumberRecord;

fn main() {
    let mut index: Apotheosis<SimpleNumberRecord, NormalDistance> = Apotheosis::new();
    index.insert(SimpleNumberRecord::create("10".to_string()).unwrap());
    index.insert(SimpleNumberRecord::create("20".to_string()).unwrap());
    index.insert(SimpleNumberRecord::create("30".to_string()).unwrap());

    index.dump("my_model.bin").expect("dump failed");

    // The M, M0, EF, HEURISTIC type parameters must all match the file; mismatches are a hard error.
    let loaded: Apotheosis<SimpleNumberRecord, NormalDistance> =
        Apotheosis::load("my_model.bin").expect("load failed");

    let results = loaded.search(&15_u32, 2, Some(50));
    for (distance, record) in &results {
        println!("distance={} id={}", distance, record.id);
    }
}
```

#### Example 4: Custom distance function

```rust
use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::DistanceAlgorithm;
use apotheosis2::datalayer::record::SimpleNumberRecord;

struct SaturatingSquaredDiff;

impl DistanceAlgorithm<u32> for SaturatingSquaredDiff {
    fn calculate_distance(&self, a: &u32, b: &u32) -> u32 {
        let diff = (*a as i64 - *b as i64).unsigned_abs(); // u64
        diff.saturating_mul(diff).min(u32::MAX as u64) as u32
    }
}

// Default is required by the Apotheosis where-clause: D: DistanceAlgorithm<...> + Default.
impl Default for SaturatingSquaredDiff {
    fn default() -> Self {
        SaturatingSquaredDiff
    }
}

fn main() {
    let mut index: Apotheosis<SimpleNumberRecord, SaturatingSquaredDiff> = Apotheosis::new();
    index.insert(SimpleNumberRecord::create("42".to_string()).unwrap());
    index.insert(SimpleNumberRecord::create("100".to_string()).unwrap());
    index.insert(SimpleNumberRecord::create("200".to_string()).unwrap());

    let results = index.search(&60_u32, 2, Some(50));
    for (distance, record) in &results {
        println!("distance={} id={}", distance, record.id);
    }
}
```
