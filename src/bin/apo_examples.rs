use apotheosis2::controllers::apotheosis::Apotheosis;
use apotheosis2::datalayer::algorithms::{NormalDistance, TlshDistance};
use apotheosis2::datalayer::record::{ApotheosisRecord, FileRecord, SimpleNumberRecord};
use std::str::FromStr;
use tlsh2::TlshDefault;

pub fn example_without_metadata() {
    let mut apotheosis = Apotheosis::<SimpleNumberRecord, NormalDistance, 32, 64, 64>::new();

    apotheosis.insert(SimpleNumberRecord::create("42".to_string()));
    apotheosis.insert(SimpleNumberRecord::create("100".to_string()));
    apotheosis.insert(SimpleNumberRecord::create("75".to_string()));
    apotheosis.insert(SimpleNumberRecord::create("50".to_string()));

    let query = 60u32;
    let results = apotheosis.search(&query, None, 3, None);

    println!("Nearest neighbors to 60:");
    for (distance, record) in results {
        println!("  ID: {} - Distance: {}", record.search_id(), distance);
    }

    let _ = apotheosis.draw("numbers");
}

pub fn example_with_metadata() {
    let mut apotheosis = Apotheosis::<FileRecord, TlshDistance, 32, 64, 64>::new();

    let hash1 =
        "T11B01CB13E7C453100330553083C9AF95D237109AB34E8D7C336D92B5032086A45B8371".to_string();
    let metadata1 = FileRecord {
        hash: TlshDefault::from_str(&hash1).unwrap(),
        filename: "malware_variant_1.exe".to_string(),
        file_path: "/samples/malware/variant1.exe".to_string(),
        version: "1.0.2".to_string(),
        size: 1024000,
        timestamp: 1696291200,
    };
    apotheosis.insert(metadata1);

    let hash2 =
        "T1AB414597A3C4977D05F61034B58A33BD6339C07C874AE534A8AAC03E1722C2D82B62F1".to_string();
    let metadata2 = FileRecord {
        hash: TlshDefault::from_str(&hash2).unwrap(),
        filename: "suspicious_app.exe".to_string(),
        file_path: "/samples/suspicious/app.exe".to_string(),
        version: "2.1.0".to_string(),
        size: 2048000,
        timestamp: 1696377600,
    };
    apotheosis.insert(metadata2);

    let hash3 =
        "T139F0558B6B8D97F684B7634668960BE6AB36C1342320C4889542D328930CD5A96727CC".to_string();
    let metadata3 = FileRecord {
        hash: TlshDefault::from_str(&hash3).unwrap(),
        filename: "clean_program.exe".to_string(),
        file_path: "/samples/benign/program.exe".to_string(),
        version: "3.0.1".to_string(),
        size: 512000,
        timestamp: 1696464000,
    };
    apotheosis.insert(metadata3);

    let query_hash = TlshDefault::from_str(
        "T1C9A001940A5E782665C08A2809E40AA1E05E24211126BA4B363D5DD84B5A9A5D1B511D",
    )
    .unwrap();
    let results = apotheosis.search(&query_hash, None, 5, None);

    println!("Similar files found:");
    for (distance, meta) in results {
        println!("\n  Filename: {}", meta.filename);
        println!("  Path: {}", meta.file_path);
        println!("  Version: {}", meta.version);
        println!("  Size: {} bytes", meta.size);
        println!("  TLSH Distance: {}", distance);
    }
    let _ = apotheosis.draw("files");
}

pub fn main() {
    println!("--- Example without metadata ---");
    example_without_metadata();

    println!("\n--- Example with rich metadata ---");
    example_with_metadata();
}
