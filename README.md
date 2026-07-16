# APOTHEOSIS

APOTHEOSIS (*APprOximaTe searcH systEm Of Similarity dIgeSts*) is a powerful system to perform similarity search, using Radix Tree and specialized implementation of the Hierarchical Navigable Small World (HNSW) data structure adapted for efficient nearest neighbor lookup of approximate matching hashes.

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

## Features
- Construction of APOTHEOSIS model, consisting on two data structures: Radix Tree and HNSW.
- Insertion of nodes in the system.
- K-nearest neighbor search based on similarity.
- Logging functionality for debugging and monitoring.


# System configuration parameters
In order to reach the proper balance between precision and speed, some configuration values can be modified in order to tune the performance. This configuration values have impact on HNSW data structure mainly. Values may be adjusted depending on your use case.

- *M*: specifies the maximum number of neighbors (connections) a node can have at each layer of the hierarchy higher than zero.
- *M0*: specifies the maximum number of neighbors (connections) a node can have at each layer of the hierarchy at layer zero.
- *EF*: controls the number of neighbors to explore during the construction and search phase of the HNSW graph.


## Usage
```rust
use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::TlshDistance;
use apotheosis2::datalayer::features::{FeatureType, TlshHashFeature};
use apotheosis2::datalayer::metadata::{ApotheosisMetadata};

// 1. Define your custom metadata structure
#[derive(Debug, Clone)]
pub struct BasicMetadata {
    pub description: String,
    pub timestamp: u64,
}
impl ApotheosisMetadata for BasicMetadata {
    fn get_attributes(&self) -> Vec<(String, String)> {
        vec![
            ("description".to_string(), self.description.clone()),
            ("timestamp".to_string(), self.timestamp.to_string()),
        ]
    }
}

fn main() {
    // 3. Initialize Apotheosis with desired generic parameters:
    //    <FeatureType, DistanceAlgorithm, Metadata, M, M0, EF>
    let mut apotheosis = Apotheosis::<TlshHashFeature, TlshDistance, BasicMetadata, 16, 32, 400>::new();

    let hash_string = "T1HASH...".to_string(); // Replace with real hash

    // 4. Insert items (Feature + Metadata)
    let feature = TlshHashFeature::create(hash_string);
    let metadata = BasicMetadata { 
        description: "Example item".to_string(),
        timestamp: 1234567890 
    }; 
    apotheosis.insert(feature, metadata);

    // 5. Search for nearest neighbors
    //    Arguments: (Query Hash, K neighbors, Optional EF search)
    let query_hash = "T1HASH...".to_string(); // Replace with real query hash
    let results = apotheosis.search(query_hash, 5, None);

    for (distance, id, metadata) in results {
        println!("Found neighbor {:?} with distance {}", id, distance);
        println!("Metadata associated: {:?}", metadata);
    }
}
```


## License
Licensed under the [GNU GPLv3](LICENSE) license.

## Funding support
Part of this research was supported by the Spanish National Cybersecurity Institute (INCIBE) under *Proyectos Estratégicos de Ciberseguridad -- CIBERSEGURIDAD EINA UNIZAR* and by the Recovery, Transformation and Resilience Plan funds, financed by the European Union (Next Generation).

![INCIBE_logos](misc/img/INCIBE_logos.jpg)