use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::algorithms::NormalDistance;
use crate::datalayer::features::FeatureType;
use crate::datalayer::features::NumberFeature;
use crate::datalayer::nodes::Node;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::cell::Cell; // Remove later
use std::collections::HashSet;
use std::time::Instant;

// TODO:
// - Use a variable for enter point
// - Use pointers instead of index for neighbors in nodes

pub struct Hnsw<D, F, const M: usize, const M0: usize> {
    distance_algorithm: D,
    features: Vec<F>,
    zero_layer: Vec<Node<M0>>,
    upper_layers: Vec<Vec<Node<M>>>,
    prng: StdRng,
    m: usize, // Maybe needed to remove!
    m0: usize,
    ef: usize,
    pub candidates_explored: Cell<usize>, // Remove later
    pub neighbors_explored: Cell<usize>,
}

impl<D, F, const M: usize, const M0: usize> Hnsw<D, F, M, M0>
where
    D: DistanceAlgorithm<F>,
    F: FeatureType + Clone,
{
    pub fn new(distance_algorithm: D, m: usize, m0: usize, ef: usize) -> Self {
        Self {
            distance_algorithm: distance_algorithm,
            features: vec![],
            upper_layers: vec![],
            zero_layer: vec![],
            prng: StdRng::seed_from_u64(42), // Deterministic seed
            m,
            m0,
            ef,
            candidates_explored: Cell::new(0),
            neighbors_explored: Cell::new(0),
        }
    }

    pub fn insert(&mut self, feature: F) -> bool { // TODO: Change return
        let new_level = self.random_level();
      //  println!("[I0] Inserting new feature {:?}. New level to insert is: {}", feature.get_id(), new_level);
        self.features.push(feature.clone());

        if self.zero_layer.is_empty() {
            // First node we are inserting
            // Add the zero node unconditionally. (LAYER 0)
            self.zero_layer.push(Node {
                feature_index: 0,
                next_node: 0,
                neighbors: [!0; M0],
            });

            // Add the node in higher layers (in case needed) (LAYER 1+)
            while self.upper_layers.len() < new_level {
                self.upper_layers.push(vec![Node {
                    feature_index: 0,
                    next_node: 0,
                    neighbors: [!0; M],
                }]);
            }
            // println!("-----------");
            return true;
        }

        let mut ef: usize = if new_level >= self.upper_layers.len() { self.ef } else { 1 }; // Check if this is correct
        let mut visited_neighbors: HashSet<usize> = HashSet::new();

        
        let mut enter_point = 0; // Setting EP.
        let mut score = 0;
        if self.upper_layers.len() == 0 {
            let node_feature_index = self.zero_layer.last().unwrap().feature_index;
            score = self.distance_algorithm.calculate_distance(
                self.features[node_feature_index].get_id(),
                feature.get_id(),
            ); 
            visited_neighbors.insert(node_feature_index);
        } else {
            score = self.distance_algorithm.calculate_distance(
                self.features[self.upper_layers.last().unwrap()[0].feature_index].get_id(),
                feature.get_id(),
            ); 
            visited_neighbors.insert(self.upper_layers.last().unwrap()[0].feature_index);
        }

        for layer_ix in (new_level..self.upper_layers.len()).rev() {
          //  println!("[I1] Descending to the first insertion level. Current level is: {}", layer_ix);
            let knn_neighbors = self.search_layer_knn(&feature, (enter_point, score), ef,layer_ix, &mut visited_neighbors);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors
                .iter()
                .min_by_key(|(_, priority)| *priority)
                .unwrap(); // Get the last element of the priority queue (inefficient). WE DON'T NEED TO DO THIS!
            let next_node_id = self.upper_layers[layer_ix][*nearest_neighbor_index].next_node; // From the nearest neighbor found, we go to the same node on the next layer
            enter_point = next_node_id;
            score = *nearest_neighbor_distance;
            //println!("[I2] Enter point for the next layer is: {}", enter_point);
            ef = if layer_ix == new_level {
                self.ef
            } else {
                1
            };
        }
       // println!("Layers to insert: {:?}. New level: {}. Current len: {}", layers_to_insert, new_level, self.layers.len());

        // From new_level to layer 0
        for layer_ix in (0..core::cmp::min(new_level, self.upper_layers.len())).rev() {
            let knn_neighbors = self.search_layer_knn(&feature,(enter_point, score),self.ef,layer_ix, &mut visited_neighbors);
           // println!("[I3] Inserting at layer: {}. Numbers of neighbors it will have: {:?}", layer_ix, knn_neighbors.len());
            self.add_node(&knn_neighbors, layer_ix as usize);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors
                .iter()
                .min_by_key(|(_, priority)| *priority)
                .unwrap(); // LIST IS ALREADY SORTED!! Shouldn't bee needed to do this!
            let next_node_id = self.upper_layers[layer_ix as usize][*nearest_neighbor_index].next_node; // From the nearest neighbor found, we go to the same node on the next layer
            enter_point = next_node_id;
            score = *nearest_neighbor_distance;
            // println!("[I4] Enter point for the next layer is: {}", enter_point);
        }

        // Create new layers up to new_level
        while self.upper_layers.len() <= new_level {
            let node = Node {
                next_node: self.upper_layers.last().unwrap().len() - 1,
                feature_index: self.features.len() - 1,
                neighbors: [!0; M],
            };
            self.upper_layers.push(vec![node]);
        }

       // println!("-----------");
       // self.print_layers();

        return true;
    }

    fn add_node(&mut self, new_neighbors: &Vec<(usize, u32)>, layer_idx: usize) {
        let new_node_index: usize = 0;
        let new_neighbors_sliced: Vec<(usize, u32)> = vec![];

        if layer_idx == 0 {
            new_node_index = self.zero_layer.len();
            let end_idx = std::cmp::min(self.m0, new_neighbors.len());
            new_neighbors_sliced = new_neighbors[0..end_idx].to_vec()
        } else {
            new_node_index = self.upper_layers[layer_idx].len();
            let end_idx = std::cmp::min(self.m, new_neighbors.len());
            new_neighbors_sliced = new_neighbors[0..end_idx].to_vec()
        };

        let new_node = Node {
            feature_index: self.features.len() - 1,
            next_node: if layer_idx == 0 {
                0
            } else if layer_idx == 1 {
                self.zero_layer.len()
            } else {
                self.upper_layers[layer_idx - 1].len()
            },
            neighbors: new_neighbors_sliced.clone(),
        };

        if layer_idx == 0 {
            self.zero_layer[layer_idx].push(new_node);
        } else {
            self.upper_layers[layer_idx].push(new_node);
        }


        // Use the data we already have instead of accessing the node again
        self.connect_neighbors(new_node_index, &new_neighbors_sliced, layer_idx);
    }

    fn connect_neighbors(
        &mut self,
        new_node_index: usize,
        new_neighbors: &Vec<(usize, u32)>,
        layer_idx: usize,
    ) {
        let max_neighbors = if layer_idx == 0 { self.m0 } else { self.m };
        let new_node_feature_idx = self.layers[layer_idx][new_node_index].feature_index;

        for &(neighbor_idx, _) in new_neighbors {
            let neighbor_feature_idx = self.layers[layer_idx][neighbor_idx].feature_index;

            let new_distance = self.distance_algorithm.calculate_distance(
                self.features[neighbor_feature_idx].get_id(),
                self.features[new_node_feature_idx].get_id(),
            );

            let current_neighbors = &mut self.layers[layer_idx][neighbor_idx].neighbors;

            if current_neighbors.len() < max_neighbors {
                let insert_pos =
                    current_neighbors.partition_point(|(_, dist)| *dist < new_distance);
                current_neighbors.insert(insert_pos, (new_node_index, new_distance));
            } else {
                // Get worst neighbor's distance (last element)
                let (_, worst_dist) = current_neighbors.last().unwrap();

                if new_distance < *worst_dist {
                    current_neighbors.pop(); // Remove worst (last element)
                    let insert_pos = current_neighbors.partition_point(|(_, dist)| *dist < new_distance);
                    current_neighbors.insert(insert_pos, (new_node_index, new_distance));

                }
            }
        }
    }

    fn search_layer_knn(&self, feature: &F,(enter_point, score): (usize, u32), ef: usize, layer_idx: usize,visited_neighbors: &mut HashSet<usize>) -> Vec<(usize, u32)> {
        let mut candidates: Vec<&Node<M0>> = vec![&self.layers[layer_idx][enter_point]]; 
        let mut currently_found_nearest_neighbors: Vec<(usize, u32)> = vec![(enter_point, score)];
        let time_start = Instant::now();

        
        while let Some(candidate) = candidates.pop() {
            for (neighbor, _) in candidate.neighbors.iter() {
                let neighbor_node: &Node = &self.layers[layer_idx][*neighbor];
                
                if visited_neighbors.insert(neighbor_node.feature_index) {
                    let score = self.distance_algorithm.calculate_distance(
                        self.features[neighbor_node.feature_index].get_id(),
                        feature.get_id(),
                    );

                    let pos = currently_found_nearest_neighbors.partition_point(|n| n.1 <= score);
                    if pos != ef {
                        if currently_found_nearest_neighbors.len() == ef {
                            currently_found_nearest_neighbors.pop();
                        }

                        currently_found_nearest_neighbors.insert(pos, (*neighbor, score));
                        candidates.push(neighbor_node);
                    }
                } 
                
            }
        }
        let time_elapsed = time_start.elapsed();
        println!("[S6] Search time: {:?}. Candidates explored: {:?} / Neighbors explored: {:?}",time_elapsed, self.candidates_explored, self.neighbors_explored );
        currently_found_nearest_neighbors
    }

    fn random_level(&mut self) -> usize {
        let uniform: f64 = self.prng.next_u64() as f64 / core::u64::MAX as f64;
        (-libm::log(uniform) * libm::log(self.m as f64).recip()) as usize
    }

    fn print_layers(&mut self) {
        for layer_idx in 0..self.layers.len() {
            println!("======================== LAYER {} ========================", layer_idx);
            for node in 0..self.layers[layer_idx].len() {
                print!("Node {} | Neighbors: ", node);
                let mut act_neighbors: Vec<usize> = vec![];
                for (neighbor_id, _) in &self.layers[layer_idx][node].neighbors {
                    act_neighbors.push(*neighbor_id);
                }
                act_neighbors.sort();
                for neighbor_id in &act_neighbors {
                    print!("{}, ", neighbor_id);
                }
                println!("(TOTAL: {})", act_neighbors.len());
            }
        }
    }

    pub fn knn_search(&self, feature: F, mut ef: usize, k: u32) -> Vec<(u32, u32)> where 
        F: FeatureType<IdType = u32>, // Constrain IdType to be u32
    {
        let mut visited_neighbors: HashSet<usize> = HashSet::new();
        let mut enter_point = 0;
        let mut score = self.distance_algorithm.calculate_distance(
            self.features[self.layers.last().unwrap()[0].feature_index].get_id(),
            feature.get_id(),
        ); 
        let mut knn_neighbors: Vec<(usize, u32)> = vec![];
        for layer_ix in (0..self.layers.len()).rev() {
            ef = if layer_ix == 0 {
                self.ef
            } else {
                1
            };
            knn_neighbors = self.search_layer_knn(&feature, (enter_point, score), ef,layer_ix as usize, &mut visited_neighbors);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors[0];
            let next_node_id = self.layers[layer_ix as usize][nearest_neighbor_index].next_node; // From the nearest neighbor found, we go to the same node on the next layer
            enter_point = next_node_id;
            score = nearest_neighbor_distance;
            
            //println!("{:?}", knn_neighbors);
        }

        let mut results: Vec<(u32, u32)> = vec![];
        for (i, score) in knn_neighbors {
            results.push((*self.features[i].get_id(), score))
        }

        results
        
    }


}

impl Default for Hnsw<NormalDistance, NumberFeature> {
    fn default() -> Self {
        Self::new(NormalDistance, 12, 24, 400)
    }
}
