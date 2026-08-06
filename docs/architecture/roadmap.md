# Documentation Roadmap: APOTHEOSIS2

## What is APOTHEOSIS2

APOTHEOSIS2 is a Rust library for similarity search over fuzzy hashes, aimed at binary forensic analysis. The problem it solves: given a TLSH hash of an unknown file, find the K most similar elements in a database of known hashes. Linear comparison (O(N·distance)) is not viable at scale; APOTHEOSIS2 reduces it to sublinear complexity in practice.

The solution combines an HNSW graph (*Hierarchical Navigable Small World*) for approximate nearest-neighbor search and a Radix Tree for exact lookup in O(key length). When the queried hash already exists in the index, the Radix Tree returns the result in constant time without traversing the graph. When it does not, the HNSW navigates the metric space efficiently to find the K nearest neighbors.

---

## Documented Views

| View | File | Answers |
|-------|---------|------------|
| Module | [views/module/vista-modulos.md](views/module/vista-modulos.md) | How is the code decomposed into units with defined responsibilities? What does each module depend on? |
| Component and Connector (C&C) | [views/cc/vista-cc.md](views/cc/vista-cc.md) | How does the system behave at runtime? What data flows exist between components? |
| Deployment | [views/deployment/vista-distribucion.md](views/deployment/vista-distribucion.md) | How is the library integrated, compiled, and deployed? In which environments does it operate? |

---

## Cross-cutting Documents

| Document | File | Purpose |
|----------|------|---------|
| Interface Documentation | [interfaz-apotheosis.md](interfaz-apotheosis.md) | Documents the usage boundary of the crate: the `Apotheosis` facade and its operations, the contract traits a client implements, the ready-to-use record and distance types, error handling, the variation points, and worked usage examples. Read this document to use the library rather than to modify it. |
| Quality Attributes | [quality-attributes.md](quality-attributes.md) | Documents the five quality attributes that drove design (performance, persistence integrity, reproducibility, maintainability, localized variability of the metric space), the three attributes explicitly out of scope (security, availability, horizontal scalability), and a cross-view mapping table locating each attribute in the three documented views. Read this document to understand why the architecture looks the way it does. |
| Directory and Data Dictionary | [directory.md](directory.md) | Glossary of domain and framework terms, acronym list, catalog of the central types, and bibliographic references. |

---

## Audience per View

| Role | Recommended documents | Reason |
|-----|-------------------|--------|
| Forensic researcher / analyst | C&C, Deployment | Understands what the system does and how to integrate it, without needing internal details |
| Developer integrating the library | Module (public interface), C&C | Needs to know the public API and the insertion and search flow |
| Core contributor | Module (complete), C&C, Quality Attributes | Needs to understand responsibilities, coupling, internal invariants, and the design rationale behind each architectural decision |
| Architect / reviewer | Quality Attributes, all views | Quality Attributes consolidates the design drivers and trade-offs; the views provide the evidence |

---

## Relationship Between Views

The views describe the same system from different angles. Key correspondences:

- `controllers/apotheosis.rs` (Module View: public facade) → **Apotheosis** component in the C&C View, the single entry point at runtime.
- `controllers/hnsw.rs` + `controllers/radix_tree.rs` (Module: algorithms) → two internal components of **Apotheosis** in C&C, with differentiated data flows depending on whether the search takes the exact fast-path or the ANN path.
- `datalayer/record.rs` (Module: domain contracts) → the types that flow through C&C connectors as input (`ApotheosisRecord`) and output (`Vec<(distance, &R)>`).
- The Deployment View specifies what "integrating the library" means: dependency in `Cargo.toml`, compilation with const-generics `M`/`M0`/`EF`, and model persistence to disk via `dump`/`load`.
- **Quality Attributes** (`quality-attributes.md`) sits above the three views. Its section 4 maps each design driver to one or more concrete elements across the module, C&C, and deployment views. It is the entry point for understanding why the architecture was structured the way it was.

---

| Document | Status |
|-----------|--------|
| roadmap.md (this file) | Complete |
| interfaz-apotheosis.md | Complete |
| vista-modulos.md | Complete |
| vista-cc.md | Complete |
| vista-distribucion.md | Complete |
| quality-attributes.md | Complete |
| directory.md | Complete |
