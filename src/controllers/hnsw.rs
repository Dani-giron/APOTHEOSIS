use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::algorithms::NormalDistance;
use crate::datalayer::features::FeatureType;
use crate::datalayer::features::NumberFeature;
use crate::datalayer::nodes::Node;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::cell::Cell; use std::cmp;
use core::cmp::min;
// Remove later
use std::collections::HashSet;
use std::marker::PhantomData;

pub struct Hnsw<D, F, const M: usize, const M0: usize, const EF: usize = 400> {
    features: Vec<F>,
    upper_layers: Vec<Vec<Node<M>>>,
    zero_layer: Vec<Node<M0>>,
    prng: StdRng,
    _phantom: PhantomData<D>,
    pub candidates_explored: Cell<usize>, // Remove later
    pub neighbors_explored: Cell<usize>,
}

impl<D, F, const M: usize, const M0: usize, const EF: usize> Hnsw<D, F, M, M0, EF>
where
    D: DistanceAlgorithm<F>,
    F: FeatureType,
{
    pub fn new() -> Self {
        Self {
            features: vec![],
            upper_layers: vec![],
            zero_layer: vec![],
            prng: StdRng::seed_from_u64(42), // Deterministic seed
            candidates_explored: Cell::new(0), // For debug purposes only
            neighbors_explored: Cell::new(0),
            _phantom: PhantomData, 
        }
    }

    pub fn insert(&mut self, feature: F) -> usize { // TODO: Change return
        let insertion_level = self.random_level();
        //println!("[I0] Inserting new feature {:?}. New level to insert is: {}", feature.get_id(), new_level);
        let feature_ref = self.features.len();
        self.features.push(feature);

        // The data structure is empty
        if self.zero_layer.is_empty() {
            self.initialize(insertion_level);
            return 0;
        }

        let mut visited_neighbors: HashSet<usize> = HashSet::new(); // We use feature indexes here
        let (mut enter_point, mut score) = self.get_enter_point(feature_ref, &mut visited_neighbors);

        // Descend to the first insertion level
        for layer_ix in (insertion_level..self.upper_layers.len()).rev() {
            //println!("[I1] Descending to the first insertion level. Current level is: {}", layer_ix);
            let knn_neighbors: Vec<(usize, u32)> = self.search_upper_layers(&self.features[feature_ref], (enter_point, score), 1, layer_ix + 1, &mut visited_neighbors);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors.first().unwrap();
            enter_point = self.upper_layers[layer_ix][*nearest_neighbor_index].next_node; // From the nearest neighbor found, we go to the same node on the next layer
            score = *nearest_neighbor_distance;

            //println!("[I2] Enter point for the next layer is: {}", enter_point);
        }
       // println!("Layers to insert: {:?}. New level: {}. Current len: {}", layers_to_insert, new_level, self.layers.len());

        // Insert from new_level to layer 1
        for layer_ix in (0..min(insertion_level, self.upper_layers.len())).rev() {
            let mut knn_neighbors = self.search_upper_layers(&self.features[feature_ref], (enter_point, score), EF, layer_ix + 1, &mut visited_neighbors);
           // println!("[I3] Inserting at layer: {}. Numbers of neighbors it will have: {:?}", layer_ix, knn_neighbors.len());
            self.add_node_upper(&mut knn_neighbors, layer_ix);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors.first().unwrap();
            enter_point = self.upper_layers[layer_ix][*nearest_neighbor_index].next_node; 
            score = *nearest_neighbor_distance;

            //println!("[I4] Enter point for the next layer is: {}", enter_point);
        }

        // Insert on layer 0
        let mut knn_neighbors = self.search_layer_zero(&self.features[feature_ref], (enter_point, score), EF, &mut visited_neighbors);
        self.add_node_zero(&mut knn_neighbors);

        // Create new layers up to new_level
        while self.upper_layers.len() < insertion_level {
            let node = Node {
                next_node: if self.upper_layers.len() != 0 {self.upper_layers.last().unwrap().len() - 1 } else { self.zero_layer.len() - 1} ,
                feature_index: self.features.len() - 1,
                neighbors: [!0; M],
                neighbor_distances: [!0; M],
                neighbor_count: 0
            };
            self.upper_layers.push(vec![node]);
        }

        //println!("---------------------------------------------------");
        //self.print_layers();

        return feature_ref;
    }

    pub fn initialize(&mut self, insertion_level: usize) {
        // The data structure is empty
        self.zero_layer.push(Node::new_empty(0, 0));

        // Add the node in higher layers (in case needed) (LAYER 1+)
        while self.upper_layers.len() < insertion_level {
            self.upper_layers.push(vec![ Node::new_empty(0, 0) ]);
        }    
    }

    #[inline]
    fn get_enter_point(&self, target_feature_ref: usize, visited: &mut HashSet<usize>) -> (usize, u32) {
        let (node_idx, feature_idx) = if self.upper_layers.is_empty() {
            (0, 0)
        } else {
            let entry_feature_idx = self.upper_layers.last().unwrap()[0].feature_index;
            (0, entry_feature_idx)
        };
        
        visited.insert(feature_idx);
        let distance = D::calculate_distance(
            self.features[feature_idx].get_id(),
            self.features[target_feature_ref].get_id(),
        );
        
        (node_idx, distance)
    }


    #[inline(always)]
    fn get_node_feature_index(&self, layer_idx: usize, index: usize) -> usize { // Maybe use enums?
        if layer_idx == 0 {
            self.zero_layer[index].feature_index
        } else {
            self.upper_layers[layer_idx - 1][index].feature_index
        }
    }

    #[inline(always)]
    fn get_node_neighbors(&self, layer_idx: usize, index: usize) -> &[usize] {
        if layer_idx == 0 {
            self.zero_layer[index].active_neighbors()
        } else {
            self.upper_layers[layer_idx - 1][index].active_neighbors()
        }
    }

    fn add_node_upper(&mut self, new_neighbors: &mut Vec<(usize, u32)>, layer_idx: usize) {
        let new_node_index = self.upper_layers[layer_idx].len();
        let mut neighbors = [!0; M];
        let mut neighbor_distances = [!0; M];
        new_neighbors.truncate(M);
        let n_neighbors = cmp::min(new_neighbors.len(), M);

        for i in 0..n_neighbors {
            neighbors[i] = new_neighbors[i].0.clone();
            neighbor_distances[i] = new_neighbors[i].1.clone();
        }

        let new_node = Node {
            feature_index: self.features.len() - 1,
            next_node: if layer_idx == 0 {
                    self.zero_layer.len()
                } else {
                    self.upper_layers[layer_idx - 1].len()
                },
            neighbors: neighbors,
            neighbor_distances: neighbor_distances,
            neighbor_count: n_neighbors
        };

        self.upper_layers[layer_idx].push(new_node);

        // println!("[AN] Number of new neighbors: {}", n_neighbors);

        self.connect_neighbors_upper(new_node_index, &new_neighbors, layer_idx);
    }

    fn add_node_zero(&mut self, new_neighbors: &mut Vec<(usize, u32)>) {
        let new_node_index = self.zero_layer.len();
        let mut neighbors = [!0; M0];
        let mut neighbor_distances = [!0; M0];

        new_neighbors.truncate(M0);
        let n_neighbors = cmp::min(new_neighbors.len(), M0);

        for i in 0..n_neighbors {
            neighbors[i] = new_neighbors[i].0.clone();
            neighbor_distances[i] = new_neighbors[i].1.clone();

        }
        self.zero_layer.push( Node {
            feature_index: self.features.len() - 1,
            next_node: 0,
            neighbors: neighbors,
            neighbor_distances: neighbor_distances,
            neighbor_count: n_neighbors
        });

        //println!("[A0] Add node ok. Connecting neighbors... | Feature index was: {}", self.features.len() - 1);
        self.connect_neighbors_zero( new_node_index, &new_neighbors);
    }

    fn connect_neighbors_zero(&mut self, new_node_index: usize, new_neighbors: &Vec<(usize, u32)>) {
        for &(neighbor_idx, new_distance) in new_neighbors {
            let neighbor_node = &mut self.zero_layer[neighbor_idx];
            
            if neighbor_node.neighbor_count < M0 { // There's an empty slot on the neighbor list. Add it there
                let slot = neighbor_node.neighbor_count;
                neighbor_node.neighbors[slot] = new_node_index;
                neighbor_node.neighbor_distances[slot] = new_distance;
                neighbor_node.neighbor_count += 1;
            } else { // Neighbor list is full - find worst neighbor using stored distances
                let (worst_ix, worst_distance) = neighbor_node
                    .active_distances() 
                    .iter()
                    .enumerate()
                    .max_by_key(|&(_, &distance)| distance)
                    .unwrap();
                
                if new_distance < *worst_distance {
                    neighbor_node.neighbors[worst_ix] = new_node_index;
                    neighbor_node.neighbor_distances[worst_ix] = new_distance;
                }
            }
        }
    }

    fn connect_neighbors_upper(&mut self, new_node_index: usize, new_neighbors: &Vec<(usize, u32)>, layer_idx: usize) {
        for &(neighbor_idx, new_distance) in new_neighbors {
            let neighbor_node = &mut self.upper_layers[layer_idx][neighbor_idx];
            
            if neighbor_node.neighbor_count < M {
                // Simple case: add to empty slot
                let slot = neighbor_node.neighbor_count;
                neighbor_node.neighbors[slot] = new_node_index;
                neighbor_node.neighbor_distances[slot] = new_distance;
                neighbor_node.neighbor_count += 1;
            } else {
                // Node is full - find worst neighbor using stored distances
                let (worst_ix, worst_distance) = neighbor_node
                    .active_distances() 
                    .iter()
                    .enumerate()
                    .max_by_key(|&(_, &distance)| distance) 
                    .unwrap();
                
                // Replace if new neighbor is better (shorter distance)
                if new_distance < *worst_distance {
                    neighbor_node.neighbors[worst_ix] = new_node_index;
                    neighbor_node.neighbor_distances[worst_ix] = new_distance; 
                }
            }
        }
    }


    /* 
    fn search_layer_knn(&self, feature: &F, (enter_point, score): (usize, u32), ef: usize, layer_idx: usize, visited_neighbors: &mut HashSet<usize>) -> Vec<(usize, u32)> {
        let mut candidates: Vec<usize> = vec![enter_point];
        let mut currently_found_nearest_neighbors: Vec<(usize, u32)> = vec![(enter_point, score)];
        let time_start = Instant::now();

        while let Some(candidate) = candidates.pop() {
            self.candidates_explored.set(self.candidates_explored.get() + 1);
           // println!("[S1] Requesting neighbors from node {} at layer", candidate);
            let neighbors = self.get_node_neighbors(layer_idx, candidate);

            for neighbor in neighbors {
                self.neighbors_explored.set(self.neighbors_explored.get() + 1);
                let neighbor_feature_index: usize = self.get_node_feature_index(layer_idx, *neighbor);
              //  println!("[S2] Exploring neighbor: {:?}", neighbor);

                
                if visited_neighbors.insert(neighbor_feature_index) {
                    let score = D::calculate_distance(
                        self.features[neighbor_feature_index].get_id(),
                        feature.get_id(),
                    );

                    let pos = currently_found_nearest_neighbors.partition_point(|n| n.1 <= score);
                    if pos != ef {
                      //  println!("[S3] CFNN Updated! New neighbor: {:?} at pos {}. Distance is: {:?}", *neighbor, pos, score);
                        if currently_found_nearest_neighbors.len() == ef {
                            currently_found_nearest_neighbors.pop();
                        }
                        currently_found_nearest_neighbors.insert(pos, (*neighbor, score));
                        candidates.push(*neighbor);
                    }
                } 
                
            }
        }

        let time_elapsed = time_start.elapsed();
        println!("[S6] Search time: {:?}. Candidates explored: {:?} / Neighbors explored: {:?}. Ef = {}",time_elapsed, self.candidates_explored, self.neighbors_explored, ef);
        currently_found_nearest_neighbors
    }*/

    fn search_upper_layers(&self, feature: &F, (enter_point, score): (usize, u32), ef: usize, layer_idx: usize, visited_neighbors: &mut HashSet<usize>) -> Vec<(usize, u32)> {
        let mut candidates: Vec<usize> = vec![enter_point];
        let mut currently_found_nearest_neighbors: Vec<(usize, u32)> = vec![(enter_point, score)];
        //let time_start = Instant::now();

        while let Some(candidate) = candidates.pop() {
           // self.candidates_explored.set(self.candidates_explored.get() + 1);
           // println!("[S1] Requesting neighbors from node {} at layer", candidate);
            let neighbors = self.upper_layers[layer_idx - 1][candidate].active_neighbors();

            for neighbor in neighbors {
                //self.neighbors_explored.set(self.neighbors_explored.get() + 1);
                let neighbor_feature_index: usize = self.upper_layers[layer_idx - 1][*neighbor].feature_index;

              //  println!("[S2] Exploring neighbor: {:?}", neighbor);
                
                if visited_neighbors.insert(neighbor_feature_index) {
                    let score = D::calculate_distance(
                        self.features[neighbor_feature_index].get_id(),
                        feature.get_id(),
                    );

                    let pos = currently_found_nearest_neighbors.partition_point(|n| n.1 <= score);
                    if pos != ef {
                      //  println!("[S3] CFNN Updated! New neighbor: {:?} at pos {}. Distance is: {:?}", *neighbor, pos, score);
                        if currently_found_nearest_neighbors.len() == ef {
                            currently_found_nearest_neighbors.pop();
                        }
                        currently_found_nearest_neighbors.insert(pos, (*neighbor, score));
                        candidates.push(*neighbor);
                    }
                } 
                
            }
        }

        //let time_elapsed = time_start.elapsed();
        //println!("[S6] Search time: {:?}. Candidates explored: {:?} / Neighbors explored: {:?}. Ef = {}",time_elapsed, self.candidates_explored, self.neighbors_explored, ef);
        currently_found_nearest_neighbors

    }

    fn search_layer_zero(&self, feature: &F, (enter_point, score): (usize, u32), ef: usize, visited_neighbors: &mut HashSet<usize>) -> Vec<(usize, u32)> {
        let mut candidates: Vec<usize> = vec![enter_point];
        let mut currently_found_nearest_neighbors: Vec<(usize, u32)> = vec![(enter_point, score)];
        //let time_start = Instant::now();
        //let mut elapsed_compare = 0;

        while let Some(candidate) = candidates.pop() {
           // self.candidates_explored.set(self.candidates_explored.get() + 1);
           // println!("[S1] Requesting neighbors from node {} at layer", candidate);
            let neighbors = self.zero_layer[candidate].active_neighbors();

            for neighbor in neighbors {
                //self.neighbors_explored.set(self.neighbors_explored.get() + 1);
                let neighbor_feature_index: usize = *neighbor;
              //  println!("[S2] Exploring neighbor: {:?}", neighbor);

                
                if visited_neighbors.insert(neighbor_feature_index) {
                    //let compare_start = Instant::now();
                    let score = D::calculate_distance(
                        self.features[neighbor_feature_index].get_id(),
                        feature.get_id(),
                    );
                    //elapsed_compare += compare_start.elapsed().as_nanos();

                    let pos = currently_found_nearest_neighbors.partition_point(|n| n.1 <= score);
                    if pos != ef {
                      //  println!("[S3] CFNN Updated! New neighbor: {:?} at pos {}. Distance is: {:?}", *neighbor, pos, score);
                        if currently_found_nearest_neighbors.len() == ef {
                            currently_found_nearest_neighbors.pop();
                        }
                        currently_found_nearest_neighbors.insert(pos, (*neighbor, score));
                        candidates.push(*neighbor);
                    }
                } 
                
            }
        }

        //let time_elapsed = time_start.elapsed();
        //println!("[S6] Search time: {:?}. Candidates explored: {:?} / Neighbors explored: {:?}. Ef = {} | Elapsed compare: {:?}",time_elapsed, self.candidates_explored, self.neighbors_explored, ef, elapsed_compare);
        currently_found_nearest_neighbors
    }

    fn random_level(&mut self) -> usize {
        let uniform: f64 = self.prng.next_u64() as f64 / core::u64::MAX as f64;
        (-libm::log(uniform) * libm::log(M as f64).recip()) as usize
    }
    

    pub fn print_layers(&mut self) {
        println!("======================== LAYER 0 ========================");
        for node in 0..self.zero_layer.len() {
            print!("Node {} | Neighbors: ", node);
            let mut act_neighbors: Vec<usize> = vec![];
            for neighbor_id in self.zero_layer[node].active_neighbors() {
                act_neighbors.push(*neighbor_id);
            }
            act_neighbors.sort();
            for neighbor_id in &act_neighbors {
                print!("{}, ", neighbor_id);
            }
            println!("(TOTAL: {})", act_neighbors.len());
        }

        for layer_idx in 0..self.upper_layers.len() {
            println!("======================== LAYER {} ========================", layer_idx + 1);
            for node in 0..self.upper_layers[layer_idx].len() {
                print!("Node {} | Neighbors: ", node);
                let mut act_neighbors: Vec<usize> = vec![];
                for neighbor_id in self.upper_layers[layer_idx][node].active_neighbors() {
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

    pub fn knn_search(&self, feature: &F, ef: usize, k: usize) -> Vec<(u32, &F)> {
        let mut visited_neighbors: HashSet<usize> = HashSet::new();
        let mut enter_point = 0;
        let mut score = D::calculate_distance(
            self.features[self.upper_layers.last().unwrap()[0].feature_index].get_id(),
            feature.get_id(),
        ); 

        let mut knn_neighbors: Vec<(usize, u32)>;
        for layer_ix in (0..self.upper_layers.len()).rev() {
            knn_neighbors = self.search_upper_layers(&feature, (enter_point, score), 1, layer_ix + 1 as usize, &mut visited_neighbors);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors[0];
            let next_node_id = self.upper_layers[layer_ix as usize][nearest_neighbor_index].next_node; // From the nearest neighbor found, we go to the same node on the next layer
            enter_point = next_node_id;
            score = nearest_neighbor_distance;
        }


        knn_neighbors = self.search_layer_zero(&feature, (enter_point, score), ef, &mut visited_neighbors);

        let mut results: Vec<(u32, &F)> = vec![];
        for (i, score) in knn_neighbors {
            results.push((score, &self.features[i]))
        }

        results
    }

    pub fn get_zero_layer_node(&self, index: usize) -> Vec<(u32, &F)> { // TODO: Rename this function
        let mut results: Vec<(u32, &F)> = vec![];
        results.push((0, &self.features[self.zero_layer[index].feature_index]));

        let node = &self.zero_layer[index];
        let neigbors = node.active_neighbors();

        for neighbor_index in neigbors {
            let score = D::calculate_distance(&self.features[*neighbor_index].get_id(), &self.features[node.feature_index].get_id());
            results.push((score, &self.features[*neighbor_index]));
        }

        results
        
    }
 
}

impl<const M: usize, const M0: usize> Default for Hnsw<NormalDistance, NumberFeature, M, M0> {
    fn default() -> Self {
        Self::new()
    }
    
}
