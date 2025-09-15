use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::algorithms::NormalDistance;
use crate::datalayer::features::FeatureType;
use crate::datalayer::features::NumberFeature;
use crate::datalayer::nodes::Node;
use priority_queue::PriorityQueue;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::cell::Cell; // Remove later
use std::cmp::Reverse;
use std::collections::HashSet;
use std::time::Duration;
use std::time::Instant;

pub struct Hnsw<D, F> {
    distance_algorithm: D,
    features: Vec<F>,
    layers: Vec<Vec<Node>>,
    prng: StdRng,
    m: usize,
    m0: usize,
    ef: usize,
    pub candidates_explored: Cell<usize>, // Remove later
    pub neighbors_explored: Cell<usize>,
}

impl<D, F> Hnsw<D, F>
where
    D: DistanceAlgorithm<F>,
    F: FeatureType + Clone,
{
    pub fn new(distance_algorithm: D, m: usize, m0: usize, ef: usize) -> Self {
        Self {
            distance_algorithm: distance_algorithm,
            features: vec![],
            layers: vec![vec![]],
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

        if self.layers[0].is_empty() {
            // First node we are inserting
            // Add the zero node unconditionally. (LAYER 0)
            self.layers[0].push(Node {
                feature_index: 0,
                next_node: 0,
                neighbors: vec![],
            });

            // Add the node in higher layers (in case needed) (LAYER 1+)
            while self.layers.len() < new_level {
                let new_node = Node {
                    feature_index: 0,
                    next_node: 0,
                    neighbors: vec![],
                };
                self.layers.push(vec![new_node]);
            }
            // println!("-----------");
            return true;
        }

        let mut ef = if new_level >= self.layers.len() {
            self.ef
        } else {
            1
        }; // Check if mut is needed here.

        let mut enter_point = 0;
        let mut score: u32 = self.distance_algorithm.calculate_distance(
            self.features[self.layers.last().unwrap()[0].feature_index].get_id(),
            feature.get_id(),
        ); // We need to move this
        // From existing layer level to the level we're going to insert. (Normal search until insertion level)
        let mut visited_neighbors: HashSet<usize> = HashSet::new(); // Maybe just store the node index? For better perfomance: WE ARE USING POINTERS!!
        visited_neighbors.insert(self.layers.last().unwrap()[0].feature_index);

        for layer_ix in (new_level + 1..=self.layers.len() - 1).rev() {
          //  println!("[I1] Descending to the first insertion level. Current level is: {}", layer_ix);
            let knn_neighbors = self.search_layer_knn(
                &feature,
                (enter_point, score),
                ef,
                layer_ix,
                &mut visited_neighbors,
            );
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors
                .iter()
                .min_by_key(|(_, priority)| *priority)
                .unwrap(); // Get the last element of the priority queue (inefficient)
            let next_node_id = self.layers[layer_ix][*nearest_neighbor_index].next_node; // From the nearest neighbor found, we go to the same node on the next layer
            enter_point = next_node_id;
            score = *nearest_neighbor_distance;
            //println!("[I2] Enter point for the next layer is: {}", enter_point);
            ef = if layer_ix == new_level + 1 {
                self.ef
            } else {
                1
            };
        }

        // Decide which layers we are inserting a node (just from new_level to 0)
        let max_layer_to_insert = core::cmp::min(new_level, self.layers.len() - 1);
        let mut layers_to_insert: Vec<usize> = (0..=max_layer_to_insert).rev().collect();
        //layers_to_insert.push(0); // We will always want to insert at layer 0

       // println!("Layers to insert: {:?}. New level: {}. Current len: {}", layers_to_insert, new_level, self.layers.len());

        // From new_level to layer 0
        for layer_ix in (0..=core::cmp::min(new_level, self.layers.len() - 1)).rev() {
            let knn_neighbors = self.search_layer_knn(
                &feature,
                (enter_point, score),
                self.ef,
                layer_ix,
                &mut visited_neighbors,
            );
           // println!("[I3] Inserting at layer: {}. Numbers of neighbors it will have: {:?}", layer_ix, knn_neighbors.len());
            self.add_node(&knn_neighbors, layer_ix as usize);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors
                .iter()
                .min_by_key(|(_, priority)| *priority)
                .unwrap(); // LIST IS ALREADY ORDERED!! Shouldn't bee needed to do this!
            let next_node_id = self.layers[layer_ix as usize][*nearest_neighbor_index].next_node; // From the nearest neighbor found, we go to the same node on the next layer
            enter_point = next_node_id;
            score = *nearest_neighbor_distance;
            // println!("[I4] Enter point for the next layer is: {}", enter_point);
        }

        // Create new layers up to new_level
        while self.layers.len() <= new_level {
            let node = Node {
                next_node: self.layers.last().unwrap().len() - 1,
                feature_index: self.features.len() - 1,
                neighbors: vec![],
            };
            self.layers.push(vec![node]);
        }

        //println!("-----------");
        //self.print_layers();

        return true;
    }

    fn add_node(&mut self, new_neighbors: &Vec<(usize, u32)>, layer_idx: usize) {
        //rintln!("[A] New neighbors will be: {:?}", new_neighbors);
        let new_node_index = self.layers[layer_idx].len();
        let new_neighbors_sliced = if layer_idx == 0 {
            let end_idx = std::cmp::min(self.m0, new_neighbors.len());
            new_neighbors[0..end_idx].to_vec()
        } else {
            let end_idx = std::cmp::min(self.m, new_neighbors.len());
            new_neighbors[0..end_idx].to_vec()
        };

        let new_node = Node {
            feature_index: self.features.len() - 1,
            next_node: if layer_idx == 0 {
                0
            } else {
                self.layers[layer_idx - 1].len()
            },
            neighbors: new_neighbors_sliced.clone(),
        };

        self.layers[layer_idx].push(new_node);

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
                    current_neighbors.push((new_node_index, new_distance));
                }
            }

            // Keep sorted by distance (closest first). Needed?
            current_neighbors.sort_unstable_by_key(|(_, distance)| *distance);
        }
    }

    fn search_layer_knn(
        &self,
        feature: &F,
        (enter_point, score): (usize, u32),
        ef: usize,
        layer_idx: usize,
        visited_neighbors: &mut HashSet<usize>,
    ) -> Vec<(usize, u32)> {
        let mut candidates: Vec<&Node> = vec![&self.layers[layer_idx][enter_point]];
        let mut currently_found_nearest_neighbors: Vec<(usize, u32)> = vec![(enter_point, score)];
        let time_start = Instant::now();

        
        while let Some(candidate) = candidates.pop() {
            // println!("[S0] Candidates to visit: {}: | Enter point is: {}. Seen: {:?}. EF = {}", candidates.len(), enter_point, visited_neighbors, ef);

            self.candidates_explored
                .set(self.candidates_explored.get() + 1);

            for (neighbor, _) in candidate.neighbors.iter() {
                let neighbor_node: &Node = &self.layers[layer_idx][*neighbor];
                self.neighbors_explored
                    .set(self.neighbors_explored.get() + 1);

                //println!("[S2] Visiting candidate's neighbor: {}.", neighbor);
                
                if visited_neighbors.insert(neighbor_node.feature_index) {
                    let score = self.distance_algorithm.calculate_distance(
                        self.features[neighbor_node.feature_index].get_id(),
                        feature.get_id(),
                    );

                    let pos = currently_found_nearest_neighbors.partition_point(|n| n.1 <= score); // Use binary_seach?
                    if pos != ef {
                        if currently_found_nearest_neighbors.len() == ef {
                            currently_found_nearest_neighbors.pop();
                        }

                       // println!("[S3] New candidate: {:?}. Distance = {}", self.features[neighbor_node.feature_index].get_id(), score);


                        currently_found_nearest_neighbors.insert(pos, (*neighbor, score));
                        candidates.push(neighbor_node);
                    }
                } 
                
            }
        }
        let time_elapsed = time_start.elapsed();
       // println!("[S6] Search time: {:?}. Candidates explored: {:?} / Neighbors explored: {:?}",time_elapsed, self.candidates_explored, self.neighbors_explored );
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
            knn_neighbors = self.search_layer_knn(&feature, (enter_point, score), 1,layer_ix as usize, &mut visited_neighbors);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors[0];
            let next_node_id: usize = self.layers[layer_ix as usize][nearest_neighbor_index].next_node; // From the nearest neighbor found, we go to the same node on the next layer
            enter_point = next_node_id;
            score = nearest_neighbor_distance;
            
            //println!("{:?}", knn_neighbors);
        }

        knn_neighbors = self.search_layer_knn(&feature, (enter_point, score), 1,layer_ix as usize, &mut visited_neighbors);


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
