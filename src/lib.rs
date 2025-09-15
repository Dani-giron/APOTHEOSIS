

pub mod controllers; 
pub mod datalayer;

#[cfg(test)]
mod tests {
    use crate::{controllers::hnsw::Hnsw, datalayer::features::NumberFeature};
    use rand_pcg::Pcg64;
    use rand::{Rng, SeedableRng};




    #[test]
    fn it_works() {
        
        // Simple approach with thread_rng
        let mut rng = rand::thread_rng();
        let random_numbers: Vec<u32> = (0..10).map(|_| rng.gen_range(0..20)).collect();
        println!("Random numbers: {:?}", random_numbers);

        let mut features: Vec<NumberFeature> = vec![];

        for n in random_numbers {
            features.push({NumberFeature { id: n }})
        }

        let mut model = Hnsw::default();
        
        let feature = NumberFeature {
            id: 7,
        };
        let feature2 = NumberFeature {
            id: 12,
        };
        model.insert(feature);
        model.insert(feature2);

        for f in features {
            model.insert(f);
        }

        println!("--------------------------- Hey! -------------------------------------");

        assert_eq!(4, 4);
    }
}
