# Deployment View: APOTHEOSIS 2

## Table of Contents

1. [Introduction and Scope](#1-introduction-and-scope)
2. [Installation View](#2-installation-view)
   - 2.1 [Crate Structure](#21-crate-structure)
   - 2.2 [External Dependencies](#22-external-dependencies)
   - 2.3 [Disk Artifacts](#23-disk-artifacts)
   - 2.4 [Integration and Distribution](#24-integration-and-distribution)
3. [Deployment View](#3-deployment-view)
4. [Correspondence with Other Views](#4-correspondence-with-other-views)

---

## 1. Introduction and Scope

The deployment view relates the software to the non-software environment in which it operates: which artifacts are installed, in which execution environment they are deployed, and how development work is allocated across teams. Following the principle of Clements et al. (*Software Architecture: Views and Beyond*, ch. 9) of documenting only what serves the project's real stakeholders, this view is organized into two sub-views and deliberately omits a third:

- **Installation sub-view** (section 2): main body. APOTHEOSIS 2 is a Rust library with no process of its own and no independent executable artifact; all of its distribution occurs through the Rust package system (Cargo) and the artifacts it generates on disk during execution. This sub-view documents those aspects.

- **Deployment sub-view** (section 3): present but brief. As a library, there is no APOTHEOSIS process to deploy on a specific machine; the library runs inside the client application process. The sub-view describes this fact and its hardware consequences.

- **Work assignment sub-view**: deliberately omitted. APOTHEOSIS 2 is a two-person research project; there is no organizational structure that requires documenting the assignment of modules to teams.

---

## 2. Installation View

### 2.1 Crate Structure

The crate is organized in three module layers inside `src/`:

```
src/
├── lib.rs                    ← crate root; declares modules controllers and datalayer
├── controllers.rs            ← declares submodules apotheosis, hnsw, radix_tree
├── datalayer.rs              ← declares submodules algorithms, nodes, record
│
├── controllers/
│   ├── apotheosis.rs         ← public facade; single API entry point
│   ├── hnsw.rs               ← multilayer ANN index
│   └── radix_tree.rs         ← exact prefix-match index
│
├── datalayer/
│   ├── algorithms.rs         ← DistanceAlgorithm trait and concrete implementations
│   ├── nodes.rs              ← HnswNode struct (pure data, no logic)
│   └── record.rs             ← ApotheosisRecord and RadixKeyMapping traits; concrete types
│
└── bin/
    ├── test_numbers.rs       ← benchmark over synthetic u32 values; compares HNSW vs. brute force
    └── test_tlsh.rs          ← benchmark over TLSH hashes read from an external data file
```

Both binaries hardcode their dataset sizes and query counts; see the sources for the current values.

### 2.2 External Dependencies

Version requirements live in `Cargo.toml` and are not repeated here; this section documents what each dependency is for.

#### Active dependencies (used in the code)

| Dependency | Actual use |
|---|---|
| `tlsh2` | `TlshDefault` type; TLSH distance computation via `diff()` in `algorithms.rs`; hash serialization. Consumed from a git fork at `github.com/danielhuici/tlsh`, which enables the `diff` and `serde` features not available in the crates.io release |
| `serde` | Serialization and deserialization of all model structs for `dump`/`load` |
| `bincode` | Compact binary encoding of the serialized model body in `dump`/`load` |
| `serde_json` | JSON deserialization in `GenericJsonRecord`, and reading the benchmark data file in `test_tlsh` |
| `gexf` | GEXF file generation for Gephi visualization from `draw()` |
| `rand` | `StdRng` in `hnsw.rs`, used to draw the random insertion level of each new element |
| `rand_core` | Core traits required by `rand` |
| `libm` | `libm::log()` in `Hnsw::random_level()` for the insertion level computation |
| `tracing` | `debug!` macros in `hnsw.rs` tracing insertion and search |

#### Declared but unused dependencies

| Dependency | Status |
|---|---|
| `axum` | Declared, but does not appear in any source file. Infrastructure for a REST API that does not exist yet |
| `rand_pcg` | Declared, but does not appear in any source file. The graph uses `rand`'s `StdRng`, not the PCG generator |
| `tracing-subscriber` | Declared, but not used in library code. A library normally depends on the `tracing` facade only and leaves the subscriber to whoever embeds it |

> **Known technical debt:** these three dependencies add transitive weight and audit surface for functionality the crate does not have. `axum` in particular pulls in an async HTTP stack. They should be removed, or moved to the binaries that actually need them, until the corresponding functionality exists.

### 2.3 Disk Artifacts

#### Serialized model: `dump` and `load` operations

`dump(path)` produces a single binary file at the path provided by the caller. The format is:

```
Offset  Size    Content
------  ------  ---------
0       4 B     Magic bytes: "APOT"
4       4 B     M  as u32 little-endian  (max neighbors, upper layers)
8       4 B     M0 as u32 little-endian  (max neighbors, layer 0)
12      4 B     EF as u32 little-endian  (exploration factor)
16      1 B     HEURISTIC as a single byte (0 or 1)
17      var.    Apotheosis struct encoded with bincode
```

The header makes the format self-describing with respect to the compile-time graph parameters: `load` reads `M`, `M0`, `EF`, and `HEURISTIC` from the file and compares them against the const-generics the loading type was compiled with. Any mismatch returns an error naming both the stored and the expected values, so the caller can identify the problem without inspecting the binary. Only if all four match does bincode deserialization proceed. The file name is free; the library imposes no convention.

The header does not cover the record type `R` or the distance type `D`, so a file written with one distance function can be handed to a type expecting another without the header objecting.

#### Visualization files: `draw` operation

`draw(path)` produces one `.gexf` file per HNSW graph layer, with the naming pattern:

```
<stem>_layer0.gexf   ← base layer (all nodes)
<stem>_layer1.gexf   ← first upper layer
<stem>_layerN.gexf   ← Nth upper layer
```

where `<stem>` is the base name without extension of the received `path`. Example: `draw("model/graph.gexf")` produces `model/graph_layer0.gexf`, `model/graph_layer1.gexf`, etc. The number of files depends on the graph height built during insertion.

The content is GEXF XML with a node attribute schema added manually as a workaround for a limitation of the `gexf` crate, which does not natively support schema declaration.

#### Benchmark data file

`test_tlsh` requires an `output_hashes.json` file in the execution directory, with the structure:

```json
{ "hashes": ["T1...", "T1...", ...] }
```

The binary slices this array by hardcoded index ranges to build its dataset and its query set, and the query range starts well beyond the dataset range. A file that merely holds enough hashes for the dataset will therefore panic on the query slice. Check the index ranges in `src/bin/test_tlsh.rs` before preparing the file.

This file is not part of the library; it is an external data artifact needed only to run the TLSH benchmark with real hashes.

### 2.4 Integration and Distribution

APOTHEOSIS 2 produces two artifact types when compiled:

- **A static Rust library** (`rlib`), the main artifact, consumed by the client application.
- **Two development binaries** (`test_numbers`, `test_tlsh`), compiled with `cargo run --bin <name>`; not deliverables.

**Not published on crates.io.** The `tlsh2` dependency is consumed from a private GitHub fork via `git = "https://github.com/danielhuici/tlsh"`, which prevents direct publication in the public registry since crates.io does not allow git dependencies.

**Integration in client projects:**

```toml
# Git dependency (distribution)
[dependencies]
apotheosis2 = { git = "https://github.com/reverseame/APOTHEOSIS", branch = "apotheosis2" }

# Local dependency (development against a working copy)
[dependencies]
apotheosis2 = { path = "../APOTHEOSIS" }
```

---

## 3. Deployment View

APOTHEOSIS 2 has no process of its own. As a Rust library, it runs inside the client application process that integrates it as a dependency. There is no APOTHEOSIS server, daemon, or container; the library is statically linked into the client binary at compile time.

### Execution Model

The full system state (HNSW graph `features[]`, `upper_layers[]`, `zero_layer[]`; RadixTree; record vector `records[]`) resides entirely in RAM at runtime. There is no disk paging or database access during search or insertion. The only operations that touch disk are `dump`, `load`, and `draw`, which are explicit and synchronous.

### Dominant Hardware Requirement

The dominant hardware requirement is RAM sufficient to hold the full collection in all three structures simultaneously. Memory footprint grows with the number of inserted elements and with the `M`, `M0` parameters (which determine the number of edges per node). No memory measurements for concrete collections are available.

### Note on the Brevity of This Sub-view

This sub-view is deliberately brief. With no process of its own, there is no deployment topology, port assignment, network configuration, or operating system requirements to document beyond those imposed by the client application. All the distribution complexity of the system resides in the installation sub-view (section 2).

### Platform

The README documents build and execution commands exclusively in bash (Unix) syntax. No explicit Windows or macOS support is declared in the repository. The `tlsh2` dependency, consumed from a private GitHub fork, may have its own platform requirements (e.g. C compilation dependencies) that are not documented in this repository. System evaluation experiments were conducted on Linux; the exact platform requirements to reproduce them are not specified in the code or documentation.

---

## 4. Correspondence with Other Views

| Element in this view | Corresponding element | View |
|---|---|---|
| `src/` tree with modules `datalayer/` and `controllers/` | Module catalog with responsibilities, interfaces, and dependencies | `vista-modulos.md` §2 |
| `serde` + `bincode` dependencies | `dump(path)` and `load(path)` operations on Apotheosis `persistence` port | `vista-cc.md` §2.2 connector `apo↔fs-io` |
| `gexf` dependency | `draw(path)` operation and connector `apo→fs-gexf` | `vista-cc.md` §2.2 connector `apo→fs-gexf` |
| `<stem>_layer<N>.gexf` artifact | FileSystem `gexf-out` port and connector `fs→gephi` | `vista-cc.md` §2.1 and §2.2 |
| `test_numbers` and `test_tlsh` binaries in `src/bin/` | ClientApplication, external role that invokes `insert` and `search` | `vista-cc.md` §2.1 ClientApplication element |
| Active dependencies (`tlsh2`, `rand`, `libm`) | Variability parameters `D` (metric), `M`, `M0`, `EF` (graph configuration) | `vista-modulos.md` §5 Variability Guide |

---

## Related Documents

- [`docs/architecture/interfaz-apotheosis.md`](../../interfaz-apotheosis.md): the crate's public API, including how to declare it and its dependencies in a client's `Cargo.toml`.
- [`docs/architecture/quality-attributes.md`](../../quality-attributes.md): documents the design drivers behind the deployment decisions in this view. Section §2.1 (performance) justifies the Cargo compilation profiles (§2.4); §2.2 (persistence integrity) justifies the binary header format (§2.3); §5.3 explains the tlsh2 git-fork trade-off (§2.2 dependencies).
- [`docs/architecture/directory.md`](../../directory.md): glossary and data dictionary for types and artifacts appearing in this view.
- [`docs/architecture/roadmap.md`](../../roadmap.md): audience guide and relationship between this view and the module and C&C views.
