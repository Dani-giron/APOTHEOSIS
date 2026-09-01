# Quality Attributes: APOTHEOSIS 2

## Table of Contents

1. [Purpose and Scope](#1-purpose-and-scope)
2. [Quality Attributes Pursued](#2-quality-attributes-pursued)
   - 2.1 [Performance](#21-performance)
   - 2.2 [Persistence Integrity](#22-persistence-integrity)
   - 2.3 [Reproducibility](#23-reproducibility)
   - 2.4 [Maintainability](#24-maintainability)
   - 2.5 [Localized Variability of the Metric Space](#25-localized-variability-of-the-metric-space)
3. [Quality Attributes out of Scope](#3-quality-attributes-out-of-scope)
4. [Mapping of Quality Attributes to Views](#4-mapping-of-quality-attributes-to-views)
5. [Systemic Trade-offs](#5-systemic-trade-offs)
   - 5.1 [Performance versus complexity of the synchrony invariant](#51-performance-versus-complexity-of-the-synchrony-invariant)
   - 5.2 [Compile-time variability versus runtime flexibility](#52-compile-time-variability-versus-runtime-flexibility)
   - 5.3 [Pure-Rust portability versus tlsh2 integration](#53-pure-rust-portability-versus-tlsh2-integration)

---

## 1. Purpose and Scope

This document collects the reasoning behind the design of APOTHEOSIS 2: what quality attributes shaped it, and what trade-offs were accepted to get them. It follows Variation 2 of §10.2.2 in Clements et al. (*Documenting Software Architectures*, 2nd ed.), which allows a documentation package to add sections beyond the standard views when the dominant design rationale warrants a dedicated place to live.

Without this document, the reasoning behind the design would be scattered across the three views: a bit in the module view, a bit in the C&C view, a bit in code comments. Here it is in one place.

**Covered:** performance, persistence integrity, reproducibility, maintainability, and localized variability of the metric space. For each, we explain the requirement that drove it, the solution that was built, and what it costs.

**Not covered:** security, availability, and horizontal scalability (section 3). Not because they don't matter, but because nothing in the current design was actually decided because of them.

---

## 2. Quality Attributes Pursued

### 2.1 Performance

#### a) Requirement and motivation

APOTHEOSIS 2 needs to find the K most similar records to a query hash in collections of millions of entries. Comparing the query against every record, O(N x distance), doesn't scale. Search has to be sublinear, without giving up too much recall. This is the driver behind almost every other design decision in the system.

#### b) Solution

**Multilayer HNSW graph.** The core search structure is a Hierarchical Navigable Small World graph (`controllers/hnsw.rs`). A search enters at the topmost layer, sparse and long-range, and descends layer by layer until it reaches the base layer, dense and short-range. The functions that implement each half of that descent are marked `#[inline]`, an explicit hint to the compiler to avoid call overhead on the innermost loops.

**RadixTree fast-path.** Before touching the graph at all, `Apotheosis::search()` checks the RadixTree. If the query hash is already indexed, neighbors come back from a single array lookup in `zero_layer` instead of a graph traversal. Repeated queries on known hashes cost O(key length) plus one constant-time lookup.

**Compile-time const-generics for graph parameters.** `Hnsw` and `HnswNode` fix their connectivity and exploration parameters (`M`, `M0`, `EF`) as const-generics. As a result, each node stores its neighbors and distances in fixed-size stack arrays instead of heap-allocated vectors, which keeps memory locality tight and avoids a heap allocation per node.

**Aggressive Cargo compilation profiles.** `Cargo.toml` sets `opt-level = 3` even in dev builds, and `codegen-units = 1` in release, so the whole crate compiles as one unit and the compiler can inline across function boundaries more aggressively.

#### c) Argument and trade-offs

Hierarchical greedy traversal gives HNSW sublinear expected search time, a result from the original algorithm's literature. The fast-path pushes that further: zero graph traversal at all for hashes already seen. The price is synchrony: three parallel structures (HNSW, RadixTree, records) now have to agree with each other, always.

The deeper trade-off is that HNSW search is approximate. For hashes not previously indexed, there's no guarantee the K neighbors returned are the true nearest ones. How close the approximation gets is controlled by `EF`, fixed at compile time; there is no runtime knob to trade precision for speed later.

---

### 2.2 Persistence Integrity

#### a) Requirement and motivation

A serialized model's internal structure depends on the const-generics (`M`, `M0`, `EF`) it was built with. Loading that file into a binary compiled with different values would produce a graph that looks fine but isn't: wrong search results, or worse, out-of-bounds memory access. This has to be caught before deserialization even starts.

#### b) Solution

`dump()` writes a small binary header before the bincode-encoded model body:

```
Offset  Size    Content
------  ------  -------
0       4 B     Magic bytes: "APOT"
4       4 B     M  as u32 little-endian
8       4 B     M0 as u32 little-endian
12      4 B     EF as u32 little-endian
16      1 B     HEURISTIC as a single byte (0 or 1)
```

`load()` reads this header before deserializing anything. It checks the magic bytes, then compares M, M0, EF, and HEURISTIC against the const-generics of the type it's loading into. Any mismatch returns an error naming exactly what didn't match, for example: `"Model parameter mismatch. File has M=16, M0=32, EF=400, HEURISTIC=false but code expects M=32, M0=64, EF=64, HEURISTIC=true"`.

#### c) Argument and trade-offs

The header makes the format self-describing about its own graph topology, at the cost of 17 extra bytes and an O(1) check. The error is actionable on its own: both the stored and expected values are right there, no need to go poking at the binary.

The check still isn't complete: it guards M, M0, EF, and HEURISTIC, but not the record type `R` or the distance type `D`. A file produced with `TlshDistance` can be handed to `load()` expecting `NormalDistance` without the header complaining. What actually happens next, a clean deserialization error or quietly wrong data, depends on how compatible the two types' serialized shapes happen to be.

---

### 2.3 Reproducibility

#### a) Requirement and motivation

HNSW assigns each inserted element to a random number of layers. That randomness shapes the resulting graph and, with it, search quality. For benchmarking and regression tests, inserting the same elements in the same order twice needs to produce the same graph both times.

#### b) Solution

`Hnsw::new()` seeds its PRNG with a fixed value (`StdRng::seed_from_u64(42)`), and the serde default used when deserializing does the same.

**Known inconsistency.** The PRNG itself isn't serialized. After a dump/load cycle, it resets to the same seed regardless of how many insertions happened before the dump. So the part of the graph built before the dump is reproducible; anything inserted after a load follows a different random sequence than it would have in an unbroken session.

#### c) Argument and trade-offs

A fixed seed means two sessions inserting the same records in the same order, without a dump/load in between, produce byte-identical graphs. That's enough for benchmarking within a single run. Not serializing the RNG state keeps the persistence format simpler and avoids a variable-size field in the header, at the cost of a behavioral discontinuity across the dump/load boundary that isn't yet called out anywhere in the public API docs.

---

### 2.4 Maintainability

#### a) Requirement and motivation

APOTHEOSIS 2 is a research library that keeps changing. A change to the metric type, the record schema, or the graph algorithm should stay local: touching one concern shouldn't force edits in unrelated modules, and the blast radius of a change should be obvious just from looking at the module structure.

#### b) Solution

**Layer separation.** The source tree splits into two layers. `datalayer/` holds pure abstractions with no algorithmic logic: the distance contract and its two implementations, the HNSW node data structure, and the record traits with their concrete types. `controllers/` holds the actual algorithms: the HNSW graph, the radix tree, and the coordinating facade.

**One-way dependencies.** Everything in `controllers/` imports from `datalayer/`. Nothing in `datalayer/` imports from `controllers/`. This is easy to check by grepping `use` statements, and it holds today.

**Facade as invariant guardian.** `apotheosis.rs` is the only module that calls both `hnsw.insert()` and `radix.insert()` as part of a single operation, and the only one that reads from both indices during `search()`. All the logic for keeping the invariant intact lives in one file.

**Encapsulation.** The `hnsw`, `radix`, and `records` fields of `Apotheosis` are private. External code cannot reach in and mutate them directly; the only way to change them is through `insert()`, which is exactly what keeps the synchrony invariant honest. This used to not be the case (the fields were `pub`), which meant the invariant was enforced by convention rather than by the compiler; that gap is now closed.

#### c) Argument and trade-offs

Because the layer boundary is one-way, a change to `HnswNode` or to `DistanceAlgorithm` only ripples upward into `controllers/`, never sideways within `datalayer/`. The facade pattern means there's exactly one file a contributor needs to understand to reason about the synchrony invariant, and now that its fields are private, the compiler enforces that nobody else can quietly break it.

---

### 2.5 Localized Variability of the Metric Space

#### a) Requirement and motivation

The primary use case is TLSH similarity, but the system also needs simpler numeric metrics for testing and benchmarking, and future applications may need other distance functions entirely. Swapping the metric shouldn't require touching the graph algorithm or the record store.

#### b) Solution

Both `Hnsw` and `Apotheosis` take a generic parameter `D: DistanceAlgorithm<R::MetricId> + Default`. To use a different distance function, instantiate a different concrete type for `D`. Two implementations ship with the library: `TlshDistance`, which computes TLSH distance via the `tlsh2` crate's `diff()`, and `NormalDistance`, plain absolute difference between two `u32`s.

This is a variation point in the Clements §6.4 sense: a well-defined spot where a decision is deferred to whoever instantiates the type. It is not general runtime modifiability.

**Limit 1.** `D` is resolved at compile time. One `Apotheosis` instance uses exactly one distance function for its whole lifetime; there's no switching it at runtime.

**Limit 2.** The `Default` bound means `D` must be constructible with no arguments. A distance function that needs configuration, say a weighted distance with a tunable weight vector, can't satisfy this bound without stashing those parameters somewhere outside the type itself (a global, a thread-local).

#### c) Argument and trade-offs

Resolving `D` at compile time lets the compiler specialize and inline every call to `calculate_distance()`, which is exactly what the performance driver in §2.1 wants. The cost is rigidity: one metric per compiled instance, and any distance function that needs construction-time parameters falls outside this variation point as it stands.

---

## 3. Quality Attributes out of Scope

**Security.** APOTHEOSIS 2 has no authentication, authorization, or isolation of its own. It's a Rust API; access control and auditing are the embedding application's job. Nothing in the codebase was decided with security in mind.

**Availability.** The library is single-process and in-memory. No replication, no failover, no health checks. If the host process crashes, anything not already written via `dump()` is gone. Availability is entirely the embedding application's concern.

**Horizontal scalability.** The whole index lives in RAM inside one process. No sharding, no distributed coordination, no partitioning of the graph across machines. Scaling up means a bigger machine, not more machines (see the deployment view's note on dominant hardware requirements).

---

## 4. Mapping of Quality Attributes to Views

Each row below is a place in the architecture where a quality attribute actually shows up. The same attribute can appear in more than one view because each view exposes a different facet of the same decision.

| Attribute | View | Concrete Element | Where to look |
|---|---|---|---|
| Performance | module | Multilayer HNSW graph (insertion and search) | `controllers/hnsw.rs`: `knn_search`, `search_upper_layers`, `search_layer_zero` |
| Performance | module | RadixTree fast-path | `controllers/apotheosis.rs`: `Apotheosis::search` |
| Performance | module | Compile-time graph parameters, fixed-size node arrays | `Hnsw` in `controllers/hnsw.rs`; `HnswNode<N>` in `datalayer/nodes.rs` |
| Performance | C&C | Search request flow, fast-path vs ANN-path branches | `vista-cc.md` Scenarios 2 and 3 |
| Performance | deployment | Cargo compilation profiles | `Cargo.toml`, `[profile.dev]` and `[profile.release]` |
| Persistence integrity | module | Header verification in dump/load | `Apotheosis::dump`, `Apotheosis::load` |
| Persistence integrity | deployment | On-disk binary format, self-describing header | `vista-distribucion.md`, "Serialized model" |
| Reproducibility | module | Fixed RNG seed at construction and on deserialize | `Hnsw::new`, `default_rng()` |
| Maintainability | module | Layer separation between `datalayer/` and `controllers/` | directory structure under `src/` |
| Maintainability | module | Traits as stable interfaces for the lower layer | `DistanceAlgorithm<ID>`, `ApotheosisRecord`, `RadixKeyMapping` |
| Maintainability | C&C | Facade pattern: `Apotheosis` as the one place the invariant can break or hold | `vista-cc.md` §2.1, §2.3, Scenario 1 |
| Localized variability | module | `DistanceAlgorithm<ID>` trait and its two implementations | `TlshDistance`, `NormalDistance` |
| Localized variability | module | Generic parameter `D` as a variation point | `vista-modulos.md` §5, Variability point 4 |

---

## 5. Systemic Trade-offs

The four decisions below cut across more than one attribute from section 2. They're different from the per-attribute trade-offs in §2.1.c through §2.5.c, which each stay within a single attribute; these are system-wide tensions.

---

### 5.1 Performance versus complexity of the synchrony invariant

**Forces in tension.** Performance (§2.1) needs the RadixTree fast-path, which needs the RadixTree, the HNSW index, and the record vector to share the same index space: whatever position `Hnsw::insert()` assigns has to be the same position stored in the RadixTree and the same position in `records[]`. Maintainability (§2.4) takes the hit, because that shared index couples three structures together, and every insertion path has to keep all three consistent at once.

**Decision.** The three structures stay in sync through a shared `usize` index, and `Apotheosis` is the only place responsible for keeping them that way.

**Evidence in code.** `Apotheosis::insert()` calls `hnsw.insert()` first, which returns the shared index, then `radix.insert()` with that index as the value, then `records.push()`. The order is documented in `vista-cc.md`, Scenario 1.

**What is gained.** A query whose hash is already indexed returns in O(key length) plus one constant-time lookup in `zero_layer`, no graph traversal at all. That's the fast-path from §2.1.

**What is sacrificed.** The insertion order is load-bearing: `hnsw.insert()` has to run before `radix.insert()` because it's the one assigning the shared index. Swap or parallelize the two and the RadixTree ends up storing the wrong index. Any future code path touching one of the three structures has to touch the other two as well, correctly, every time.

---

### 5.2 Compile-time variability versus runtime flexibility

**Forces in tension.** Localized variability (§2.5) wants the distance function swappable without touching the graph algorithm. Performance (§2.1) wants that swap resolved at compile time, so the compiler can specialize and inline `calculate_distance()` everywhere it's called. Both pull toward a compile-time parameter, which pulls away from runtime flexibility: a compile-time parameter can't change while the program runs, and the `Default` bound narrows which distance types are even admissible.

**Decision.** `D` is a compile-time generic on both `Hnsw` and `Apotheosis`. Whoever instantiates the type picks a concrete `D`, and the compiler produces a fully specialized binary for that choice.

**Evidence in code.** `TlshDistance` and `NormalDistance`, both in `datalayer/algorithms.rs`, satisfy the `DistanceAlgorithm + Default` bound and are used interchangeably on the same generic graph code.

**What is gained.** Distance computation costs nothing at runtime beyond what a hand-written specialized function would cost. The codebase already runs two distinct metrics through the same graph code path without duplicating it.

**What is sacrificed.** One `Apotheosis` instance is locked to one distance function for its whole life; switching means building a new instance. Distance functions needing construction-time parameters don't fit this bound cleanly.

---

### 5.3 Pure-Rust portability versus tlsh2 integration

**Forces in tension.** Depending only on crates.io packages would make `apotheosis2` itself publishable there, and easier to audit. But TLSH distance via `diff()` and serde support for `TlshDefault` are both needed, and the crates.io release of `tlsh2` doesn't have either.

**Decision.** Depend on a private git fork of `tlsh2` that adds the `diff` and `serde` features.

**Evidence in code.** `Cargo.toml` points `tlsh2` at `github.com/danielhuici/tlsh` with those two features enabled. `TlshDistance::calculate_distance()` uses `diff()`; the serde feature is what makes `Apotheosis::dump()`/`load()` work at all for TLSH-based records.

**What is gained.** TLSH distance works, and models serialize end to end, which is what persistence integrity (§2.2) needs. No workaround or intermediate representation needed anywhere in the search or serialization paths.

**What is sacrificed.** `apotheosis2` can't be published to crates.io while this git dependency stays, since crates.io doesn't accept git dependencies. Anyone integrating `apotheosis2` via git inherits this same transitive dependency on the fork. Whatever platform-specific build requirements the fork has aren't documented here; they propagate silently to every caller.
