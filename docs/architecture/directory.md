# Directory and Data Dictionary: APOTHEOSIS 2

## Table of Contents

1. [Glossary](#1-glossary)
2. [Acronyms](#2-acronyms)
3. [Data Dictionary](#3-data-dictionary)
4. [References](#4-references)

---

This document is the Directory section of the APOTHEOSIS 2 documentation package, in the sense of Clements et al. (*Documenting Software Architectures*, 2nd ed., §10.2.1 Section 6). It serves two purposes: a glossary of conceptual and domain terms used across the views and the beyond-views documents, and a data dictionary that catalogs the central types of the system for readers who need a quick reference without reading the full view documents. It complements the roadmap (`roadmap.md`) and the quality attributes document (`quality-attributes.md`) as part of the documentation beyond views.

---

## 1. Glossary

**ANN (Approximate Nearest Neighbor).** The family of algorithms that return neighbors close to the optimum without guaranteeing they are exactly the K nearest. ANN algorithms trade exactness for speed, making sublinear search tractable over large collections. APOTHEOSIS 2 uses HNSW as its ANN algorithm; the degree of approximation is governed by the compile-time parameter `EF`.

**Beyond views.** In the Views and Beyond framework (Clements et al., §10.2), the documentation content that applies to more than one view or to the package as a whole, rather than to a single view. Examples include the roadmap, the quality attributes document, and this directory. Beyond-views documents are not redundant with the views; they consolidate information that would otherwise be scattered across multiple view documents.

**Binding time.** The point in the development or deployment lifecycle at which a variability point is resolved. In APOTHEOSIS 2, all variability points (the distance function `D`, and the graph parameters `M`, `M0`, `EF`) have binding time at compile time. No variability is resolved at runtime.

**C&C view.** See Component-and-Connector view.

**Component-and-Connector view.** An architectural view that documents the system at runtime, in terms of active components and the connectors between them. For APOTHEOSIS 2, this view is documented in `docs/architecture/views/cc/vista-cc.md`. It describes the Apotheosis, Hnsw, and RadixTree components and the call-return connectors between them.

**Const-generics.** In Rust, generic parameters that are compile-time constant values rather than types. APOTHEOSIS 2 uses const-generics `M`, `M0`, and `EF` to fix the HNSW graph topology at compile time. Resolving these parameters at compile time allows the compiler to allocate fixed-size arrays on the stack (`HnswNode<N>`) and to specialize the graph code for the concrete connectivity.

**Deployment view.** An architectural view that relates the software to its non-software environment, including installation artifacts, external dependencies, disk artifacts, and execution platform. For APOTHEOSIS 2, this view is documented in `docs/architecture/views/deployment/vista-distribucion.md`.

**Fast-path.** In APOTHEOSIS 2, the code path in `Apotheosis::search()` (`src/controllers/apotheosis.rs`) that returns results in O(key length) when the queried hash is already present in the RadixTree, bypassing HNSW graph traversal entirely. The fast-path is activated only when the record type implements `RadixKeyMapping` and returns a non-`None` radix key. When the RadixTree lookup returns `None`, search falls through to full HNSW traversal.

**Fuzzy hash.** A hash function whose output preserves locality: small changes in the input produce small changes in the output. This property enables similarity comparison between inputs without exact matching, which is not possible with cryptographic hashes. TLSH is the fuzzy hash used as the primary metric in APOTHEOSIS 2.

**GEXF (Graph Exchange XML Format).** An XML-based file format for representing graph structures, consumed by visualization tools such as Gephi. The `Apotheosis::draw()` method (`src/controllers/apotheosis.rs`) produces one GEXF file per HNSW layer with the naming pattern `<stem>_layer<N>.gexf`. The `gexf` crate (version 0.1.1, `Cargo.toml`) is used to generate these files.

**HNSW (Hierarchical Navigable Small World).** A multilayer graph structure for approximate nearest-neighbor search, introduced by Malkov and Yashunin (2018). See section 4 for the reference. Search enters at the topmost layer (sparse, long-range connections) and descends layer by layer, narrowing the candidate set at each level until reaching the base layer. In APOTHEOSIS 2, the HNSW graph is implemented in `src/controllers/hnsw.rs`.

**kNN (k-Nearest Neighbors).** A query that returns the K elements closest to a query point under a given distance metric. In APOTHEOSIS 2, `Hnsw::knn_search()` (`src/controllers/hnsw.rs`) answers kNN queries approximately via HNSW traversal.

**Layer zero.** In HNSW, the base layer of the graph, present for every inserted element. Layer zero has higher connectivity than the upper layers (governed by `M0` rather than `M`) and is the layer where the final, highest-precision neighbor search takes place. In APOTHEOSIS 2, layer zero is stored in `Hnsw.zero_layer` as `Vec<HnswNode<M0>>`.

**Module view.** An architectural view that documents the static decomposition of the system into modules, their responsibilities, and their dependency relationships. For APOTHEOSIS 2, this view is documented in `docs/architecture/views/module/vista-modulos.md`. It includes the uses view, the variability guide, and the design rationale stubs.

**Monomorphization.** In Rust, the process by which the compiler produces a separate, fully specialized binary for each concrete instantiation of a generic type or function. Monomorphization eliminates runtime dispatch overhead for generic code. In APOTHEOSIS 2, monomorphization is the mechanism that makes the compile-time variation point `D` (the distance function) zero-cost at runtime.

**RadixTree.** A prefix tree structure that finds exact matches by byte key in O(key length). In APOTHEOSIS 2, the RadixTree (implemented as `RadixNode<u8, Option<usize>>` in `src/controllers/radix_tree.rs`) is used as the exact-match index for hashes already present in the system. It stores the shared `usize` index that links each hash to its corresponding position in the HNSW graph and the record vector.

**Recall.** In information retrieval, the fraction of relevant results that a search actually returns. In ANN search, recall is typically defined as the fraction of the true K nearest neighbors that the approximate algorithm finds. APOTHEOSIS 2 trades perfect recall for sublinear search time; the degree of approximation is governed by `EF`.

**Shared index.** In APOTHEOSIS 2, the common `usize` value that links a record across the three parallel internal structures: `hnsw.features[i]`, `radix[key] = Some(i)`, and `records[i]`. The shared index is assigned by `Hnsw::insert()` and stored in the RadixTree by `Apotheosis::insert()`. All three structures must reference the same element via this index; this constraint is the synchrony invariant.

**Synchrony invariant.** In APOTHEOSIS 2, the property that the HNSW graph, the RadixTree, and the records vector are always indexed consistently by a shared `usize` (see "Shared index"). The invariant must hold after every `insert` operation and after every `load` operation. It is maintained by `Apotheosis::insert()` (`src/controllers/apotheosis.rs`) and is documented in `docs/architecture/views/module/vista-modulos.md` §3.3.

**TLSH (Trend Micro Locality Sensitive Hash).** A fuzzy hash designed for similarity comparison between binary artifacts, introduced by Oliver et al. (2013). See section 4 for the reference. TLSH produces a 35-byte binary digest, typically rendered as a 70-character hexadecimal string prefixed by "T1"; the distance between two TLSH hashes is computed via the `diff()` method. In APOTHEOSIS 2, TLSH distance is implemented in `TlshDistance::calculate_distance()` (`src/datalayer/algorithms.rs`), using the `tlsh2` crate consumed from a private git fork.

**Variation point.** In the Views and Beyond framework (Clements et al., §6.4), a well-defined location in the architecture where a design decision is deferred to the instantiator or to compile time. In APOTHEOSIS 2, the primary variation point is the distance function `D`, which can be any type implementing `DistanceAlgorithm<R::MetricId>`. The three const-generics `M`, `M0`, and `EF` are additional variation points. All four have binding time at compile time.

---

## 2. Acronyms

ADR: Architectural Decision Record

ANN: Approximate Nearest Neighbor

API: Application Programming Interface

C&C: Component and Connector

DFRWS: Digital Forensics Research Workshop

GEXF: Graph Exchange XML Format

HNSW: Hierarchical Navigable Small World

kNN: k-Nearest Neighbors

QA: Quality Attribute

RNG: Random Number Generator

SEI: Software Engineering Institute

TLSH: Trend Micro Locality Sensitive Hash

V&B: Views and Beyond

---

## 3. Data Dictionary

---

### `Apotheosis<R, D, M, M0, EF, HEURISTIC>`

**Purpose.** The public facade of the library. Coordinates the HNSW graph, the RadixTree, and the records vector, maintains the synchrony invariant across all three, and exposes the public API (`insert`, `search`, `dump`, `load`, `draw`).

**Generic parameters.**
- `R: ApotheosisRecord`: the domain record type. Defines the associated `MetricId` used for distance computation and the optional radix key for exact-match lookup.
- `D: DistanceAlgorithm<R::MetricId> + Default`: the distance function, resolved at compile time.
- `const M: usize` (default 16): maximum neighbors per node in the HNSW upper layers.
- `const M0: usize` (default 32): maximum neighbors per node in HNSW layer zero.
- `const EF: usize` (default 400): exploration factor during HNSW construction and search.

**Constraints.** The fields `hnsw`, `radix`, and `records` are private; the only way to mutate them is through `insert()`, which is what keeps the synchrony invariant intact. The `dump` method requires `Self: serde::Serialize`; `load` requires `Self: serde::de::DeserializeOwned`. A file produced with one set of `M`, `M0`, `EF`, `HEURISTIC` values cannot be loaded into an instance compiled with different values.

**Defined in.** `src/controllers/apotheosis.rs`.

**See also.** `docs/architecture/views/module/vista-modulos.md` §2.1 (entry for `controllers/apotheosis.rs`); `docs/architecture/views/cc/vista-cc.md` §2.1 (Apotheosis component and its ports); `docs/architecture/quality-attributes.md` §5.1 (synchrony invariant trade-off) and §2.4 (maintainability, facade as invariant guardian).

---

### `Hnsw<D, ID, M, M0, EF, HEURISTIC>`

**Purpose.** The multilayer approximate nearest-neighbor index. Organizes metric identifiers in a Hierarchical Navigable Small World graph and answers kNN queries by descending from the topmost layer to layer zero. Also supports direct neighbor retrieval by shared index, which is the mechanism used by the fast-path.

**Generic parameters.**
- `D: DistanceAlgorithm<ID> + Default`: the distance function used for all node comparisons during insertion and search.
- `ID`: the metric identifier type stored in the graph (equals `R::MetricId` when used through `Apotheosis`).
- `const M: usize`: maximum neighbors per node in the upper layers.
- `const M0: usize`: maximum neighbors per node in layer zero.
- `const EF: usize` (default 400): exploration factor; controls the size of the candidate list during construction and search.

**Constraints.** `D` must implement `Default` to allow construction with `Hnsw::new()`. The `prng` field (`StdRng`) is excluded from serialization (`#[serde(skip, default = "default_rng")]`); after a `dump`/`load` cycle the RNG resets to seed 42 regardless of how many insertions occurred before the dump. The zero-layer and upper-layer neighbor indices use different index spaces: `zero_layer` neighbors are feature indices into `features[]`; upper-layer neighbors are node indices within that layer.

**Defined in.** `src/controllers/hnsw.rs`.

**See also.** `docs/architecture/views/module/vista-modulos.md` §2.1 (entry for `controllers/hnsw.rs`); `docs/architecture/views/cc/vista-cc.md` §2.1 (Hnsw component); `docs/architecture/quality-attributes.md` §2.1 (performance), §2.3 (reproducibility), §5.2 (compile-time variability).

---

### `HnswNode<N>`

**Purpose.** A single node in the HNSW graph. Stores the fixed-size neighbor list and distances for one layer, plus pointers to the corresponding node in the next lower layer and to the element in `Hnsw.features[]`.

**Generic parameters.**
- `const N: usize`: the maximum number of neighbors this node can store. Instantiated as `HnswNode<M>` in upper layers and `HnswNode<M0>` in layer zero.

**Constraints.** The `neighbors` and `neighbor_distances` arrays have fixed capacity `N` allocated on the stack. Only the first `neighbor_count` entries are valid; `active_neighbors()` and `active_distances()` return slices of that prefix. Unused slots are initialized to `u32::MAX`. Serialization of the const-generic arrays is handled by an internal `serde_array` helper module because serde does not support const-generic arrays directly.

**Defined in.** `src/datalayer/nodes.rs`.

**See also.** `docs/architecture/views/module/vista-modulos.md` §2.1 (entry for `datalayer/nodes.rs`); `docs/architecture/quality-attributes.md` §2.1 (stack allocation of neighbor arrays as a performance decision).

---

### `RadixNode<K, V>`

**Purpose.** A node in a generic radix (prefix) tree. Each node stores the path segment that leads to it, an optional value at that path, and a list of children indexed by the first byte of their respective path segments. In APOTHEOSIS 2, instantiated as `RadixNode<u8, Option<usize>>` to map hash byte sequences to shared HNSW indices.

**Generic parameters.**
- `K`: the key element type. Must satisfy `Copy + PartialEq + PartialOrd` for the `insert` and `find` methods to compile.
- `V`: the value type stored at each node. Must satisfy `Clone` for the `insert` and `find` methods.

**Constraints.** The struct declaration itself carries no bounds; the bounds `K: Copy + PartialEq + PartialOrd` and `V: Clone` are required by the `impl` block. A `RadixNode` created with `RadixNode::new(vec![], None)` represents the root of an empty tree, which is how `Apotheosis::new()` initializes `self.radix`.

**Defined in.** `src/controllers/radix_tree.rs`.

**See also.** `docs/architecture/views/module/vista-modulos.md` §2.1 (entry for `controllers/radix_tree.rs`); `docs/architecture/views/cc/vista-cc.md` §2.1 (RadixTree component); `docs/architecture/quality-attributes.md` §2.1 (fast-path), §5.1 (synchrony invariant).

---

### `DistanceAlgorithm<ID>`

**Purpose.** The trait that defines the distance contract between two metric values. Any type implementing this trait can serve as the distance function `D` in `Hnsw` and `Apotheosis`. The trait has a single required method.

**Generic parameters.**
- `ID`: the type of the metric identifiers being compared (for example, `u32` or `TlshDefault`).

**Constraints.** Implementors must also implement `Default` to satisfy the bound on `Hnsw` and `Apotheosis`. `calculate_distance` must return a `u32`; distances are compared by magnitude throughout the graph algorithm, so the return value must be consistent with the desired ordering.

**Defined in.** `src/datalayer/algorithms.rs`.

**See also.** `docs/architecture/views/module/vista-modulos.md` §2.1 (entry for `datalayer/algorithms.rs`) and §5 Variability point 4; `docs/architecture/quality-attributes.md` §2.5 (localized variability of the metric space) and §5.2 (compile-time variability versus runtime flexibility).

---

### `ApotheosisRecord`

**Purpose.** The primary trait for any entity that can be inserted into the system. Defines how a domain object exposes its metric identifier for HNSW indexing and its optional key-value attributes for GEXF export.

**Generic parameters.** None. The trait uses an associated type.

**Constraints.**
- The associated type `MetricId` must satisfy `Clone + RadixKeyMapping`.
- `search_id(&self) -> Self::MetricId` is required and must return the value used for distance computation in the HNSW graph.
- `get_attributes(&self) -> Vec<(String, String)>` has a default implementation that returns an empty vector; implementors may override it to supply metadata for GEXF visualization.
- A known design limitation: `get_attributes()` is a GEXF export concern embedded in the domain trait, which violates separation of concerns (noted in `docs/architecture/views/module/vista-modulos.md` §2.1, entry for `datalayer/record.rs`).

**Defined in.** `src/datalayer/record.rs`.

**See also.** `docs/architecture/views/module/vista-modulos.md` §2.1 (entry for `datalayer/record.rs`).

---

### `RadixKeyMapping`

**Purpose.** An optional extension trait on metric identifier types. Specifies whether a metric identifier can be represented as a byte sequence for exact-match lookup in the RadixTree. Types that implement this trait and return `Some(...)` from `to_radix_key()` enable the fast-path in `Apotheosis::search()`.

**Generic parameters.** None.

**Constraints.** The default implementation of `to_radix_key()` returns `None`, meaning that metric types which do not implement this method explicitly are not eligible for the fast-path; all their searches go through full HNSW traversal. In the current codebase, `TlshDefault` and `u32` both provide non-`None` implementations in `src/datalayer/record.rs`.

**Defined in.** `src/datalayer/record.rs`.

**See also.** `docs/architecture/views/module/vista-modulos.md` §2.1 (entry for `datalayer/record.rs`); `docs/architecture/quality-attributes.md` §2.1 (fast-path mechanism).

---

### `NormalDistance`

**Purpose.** A zero-sized concrete implementation of `DistanceAlgorithm<u32>` that computes the absolute difference between two `u32` values. Used in benchmark binaries and correctness tests over numeric collections.

**Generic parameters.** None.

**Constraints.** Derives `Default` and `serde::{Serialize, Deserialize}`. Does not derive `Clone` (unlike `TlshDistance`). Implements `DistanceAlgorithm<u32>` via `a.abs_diff(*b)`.

**Defined in.** `src/datalayer/algorithms.rs`.

**See also.** `docs/architecture/quality-attributes.md` §2.5 (localized variability of the metric space); `docs/architecture/views/module/vista-modulos.md` §5 Variability point 4.

---

### `TlshDistance`

**Purpose.** A zero-sized concrete implementation of `DistanceAlgorithm<TlshDefault>` that computes TLSH distance between two `TlshDefault` values via the `diff()` method from the `tlsh2` crate. This is the primary distance function for the binary forensics use case.

**Generic parameters.** None.

**Constraints.** Derives `Default`, `Clone`, and `serde::{Serialize, Deserialize}`. Implements `DistanceAlgorithm<TlshDefault>` via `a.diff(&b, true)`, where the boolean argument enables length-difference inclusion in the distance. Requires the `diff` and `serde` features of the `tlsh2` crate, which are available only in the private git fork declared in `Cargo.toml`.

**Defined in.** `src/datalayer/algorithms.rs`.

**See also.** `docs/architecture/quality-attributes.md` §2.5 (localized variability) and §5.3 (portability trade-off); `docs/architecture/views/module/vista-modulos.md` §5 Variability point 4.

---

## 4. References

Clements, P., Bachmann, F., Bass, L., Garlan, D., Ivers, J., Little, R., Merson, P., Nord, R., and Stafford, J. *Documenting Software Architectures: Views and Beyond*, 2nd ed. Addison-Wesley, 2010.

Malkov, Y. A., and Yashunin, D. A. "Efficient and robust approximate nearest neighbor search using Hierarchical Navigable Small World graphs." IEEE Transactions on Pattern Analysis and Machine Intelligence, vol. 42, no. 4, pp. 824-836, 2020. First circulated as an arXiv preprint (arXiv:1603.09320) in 2016, last revised in 2018.

Oliver, J., Cheng, C., and Chen, Y. "TLSH: A Locality Sensitive Hash." In *Proceedings of the 4th Cybercrime and Trustworthy Computing Workshop*, 2013.
