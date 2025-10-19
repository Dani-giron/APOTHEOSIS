use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::{NormalDistance, TlshDistance};
use apotheosis2::datalayer::features::{FeatureType, TlshHashFeature};
use apotheosis2::datalayer::metadata::FileMetadata;
use apotheosis2::{datalayer::features::NumberFeature};


pub fn example_without_metadata() {
    let mut apotheosis: Apotheosis<NumberFeature, NormalDistance> = Apotheosis::new();
    
    apotheosis.insert(NumberFeature::create("42".to_string()), ());
    apotheosis.insert(NumberFeature::create("100".to_string()), ());
    apotheosis.insert(NumberFeature::create("75".to_string()), ());
    apotheosis.insert(NumberFeature::create("50".to_string()), ());
    
    let results = apotheosis.search("60".to_string(), 3);
    
    println!("Nearest neighbors to 60:");
    for (distance, id, _) in results {
        println!("  ID: {} - Distance: {}", id, distance);
    }

    apotheosis.draw("numbers");

}

pub fn example_with_metadata() {
    let mut apotheosis: Apotheosis<TlshHashFeature, TlshDistance, FileMetadata> = Apotheosis::new();
    
    let hash1 = "T11B01CB13E7C453100330553083C9AF95D237109AB34E8D7C336D92B5032086A45B8371".to_string();
    let feature1 = TlshHashFeature::create(hash1);
    let metadata1 = FileMetadata {
        filename: "malware_variant_1.exe".to_string(),
        file_path: "/samples/malware/variant1.exe".to_string(),
        version: "1.0.2".to_string(),
        size: 1024000,
        timestamp: 1696291200,
    };
    apotheosis.insert(feature1, metadata1);
    
    let hash2 = "T1AB414597A3C4977D05F61034B58A33BD6339C07C874AE534A8AAC03E1722C2D82B62F1".to_string();
    let feature2 = TlshHashFeature::create(hash2);
    let metadata2 = FileMetadata {
        filename: "suspicious_app.exe".to_string(),
        file_path: "/samples/suspicious/app.exe".to_string(),
        version: "2.1.0".to_string(),
        size: 2048000,
        timestamp: 1696377600,
    };
    apotheosis.insert(feature2, metadata2);
    
    let hash3 = "T139F0558B6B8D97F684B7634668960BE6AB36C1342320C4889542D328930CD5A96727CC".to_string();
    let feature3 = TlshHashFeature::create(hash3);
    let metadata3 = FileMetadata {
        filename: "clean_program.exe".to_string(),
        file_path: "/samples/benign/program.exe".to_string(),
        version: "3.0.1".to_string(),
        size: 512000,
        timestamp: 1696464000,
    };
    apotheosis.insert(feature3, metadata3);
    
    let query_hash = "T1C9A001940A5E782665C08A2809E40AA1E05E24211126BA4B363D5DD84B5A9A5D1B511D".to_string();
    let results = apotheosis.search(query_hash, 5);
    
    println!("Similar files found:");
    for (distance, id, meta) in results {
        println!("\n  Filename: {}", meta.filename);
        println!("  Path: {}", meta.file_path);
        println!("  Version: {}", meta.version);
        println!("  Size: {} bytes", meta.size);
        println!("  TLSH Distance: {}", distance);
        println!("  Hash: {:?}", id.hash());
    }
    apotheosis.draw("files");

}

pub fn main() {
    println!("--- Example without metadata ---");
    example_without_metadata();
    
    println!("\n--- Example with rich metadata ---");
    example_with_metadata();
}