# APOTHEOSIS

APOTHEOSIS (*APprOximaTe searcH systEm Of Similarity dIgeSts*) is a powerful system to perform similarity search, using Radix Tree and specialized implementation of the Hierarchical Navigable Small World (HNSW) data structure adapted for efficient nearest neighbor lookup of approximate matching hashes.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

## Features
- Construction of APOTHEOSIS model, consisting on two data structures: Radix Tree and HNSW.
- Insertion of nodes in the system.
- K-nearest neighbor search based on similarity.
- Model persistence: dump a built model to disk and load it back.
- GEXF export of the HNSW graph (one file per layer).
- Logging functionality for debugging and monitoring.


# System configuration parameters
In order to reach the proper balance between precision and speed, some configuration values can be modified in order to tune the performance. This configuration values have impact on HNSW data structure mainly. Values may be adjusted depending on your use case.

- *M*: specifies the maximum number of neighbors (connections) a node can have at each layer of the hierarchy higher than zero.
- *M0*: specifies the maximum number of neighbors (connections) a node can have at each layer of the hierarchy at layer zero.
- *EF*: controls the number of neighbors to explore during the construction and search phase of the HNSW graph.


## Installation
Add `apotheosis2` and its direct dependencies to `Cargo.toml`. The crate is not published on crates.io yet, so it must be declared via git or a local path:

```toml
[dependencies]
apotheosis2 = { git = "https://github.com/reverseame/APOTHEOSIS", branch = "apotheosis2" }
tlsh2 = { git = "https://github.com/danielhuici/tlsh", features = ["diff", "serde"] }
```

`tlsh2` is required directly whenever TLSH types (such as `TlshDefault`) appear in your code, for example to build a query hash.

## Usage
Implement `ApotheosisRecord` on your domain type, or use one of the ready-made types such as `SimpleTlshRecord`:

```rust
use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::TlshDistance;
use apotheosis2::datalayer::record::SimpleTlshRecord;
use std::str::FromStr;
use tlsh2::TlshDefault;

fn main() {
    let mut index: Apotheosis<SimpleTlshRecord, TlshDistance> = Apotheosis::new();

    // SimpleTlshRecord::create panics on invalid TLSH strings.
    // Replace with real TLSH hash strings produced by your data pipeline.
    index.insert(SimpleTlshRecord::create("<tlsh_hash_a>".to_string()));
    index.insert(SimpleTlshRecord::create("<tlsh_hash_b>".to_string()));

    let query = TlshDefault::from_str("<tlsh_hash_a>").expect("invalid TLSH hash");

    // Passing None for ef_search applies the library default.
    let results: Vec<(u32, &SimpleTlshRecord)> = index.search(&query, 5, None);

    for (distance, record) in &results {
        println!("distance={} radix_key={}", distance, record.radix_key);
    }
}
```

### Persistence and export

A built model can be saved to disk and loaded back. `load()` validates the M, M0, EF and HEURISTIC parameters and the distance type recorded in the file header against the loading type, and refuses to load on any mismatch. The HNSW graph can also be exported to GEXF files (one per layer).

```rust
// dump() and load() return Result.
index.dump("model.bin")?;
let loaded: Apotheosis<SimpleTlshRecord, TlshDistance> = Apotheosis::load("model.bin")?;

// export::gexf::draw() also returns Result and writes model_layer0.gexf, model_layer1.gexf, ...
apotheosis2::export::gexf::draw(&index, "model")?;
```


## License
Licensed under the [GNU GPLv3](LICENSE) license.

## Funding support
Part of this research was supported by the Spanish National Cybersecurity Institute (INCIBE) under *Proyectos Estratégicos de Ciberseguridad -- CIBERSEGURIDAD EINA UNIZAR* and by the Recovery, Transformation and Resilience Plan funds, financed by the European Union (Next Generation).

![INCIBE_logos](misc/img/INCIBE_logos.jpg)