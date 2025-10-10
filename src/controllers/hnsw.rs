use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::nodes::Node;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use tracing::debug;
use std::cmp;
use core::cmp::min;
use std::marker::PhantomData;
use std::collections::HashSet;

pub struct Hnsw<D, ID, const M: usize, const M0: usize, const EF: usize = 400> 
where
    D: DistanceAlgorithm<ID>,
{
    features: Vec<ID>,
    upper_layers: Vec<Vec<Node<M>>>,
    zero_layer: Vec<Node<M0>>,
    prng: StdRng,
    _phantom: PhantomData<D>,
}

impl<D, ID, const M: usize, const M0: usize, const EF: usize> Hnsw<D, ID, M, M0, EF>
where 
    D: DistanceAlgorithm<ID>
{
    pub fn new() -> Self {
        Self {
            features: vec![],
            upper_layers: vec![],
            zero_layer: vec![],
            prng: StdRng::seed_from_u64(42),
            _phantom: PhantomData, 
        }
    }

    pub fn insert(&mut self, feature: ID) -> usize { // TODO: Change return
        let insertion_level = self.random_level();
        debug!("[I0] Inserting new feature. New level to insert is: {}", insertion_level);
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
            debug!("[I1] Descending to the first insertion level. Current level is: {}", layer_ix);
            let knn_neighbors: Vec<(usize, u32)> = self.search_upper_layers(&self.features[feature_ref], (enter_point, score), 1, layer_ix + 1, &mut visited_neighbors);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors.first().unwrap();
            enter_point = self.upper_layers[layer_ix][*nearest_neighbor_index].next_node; // From the nearest neighbor found, we go to the same node on the next layer
            score = *nearest_neighbor_distance;

            debug!("[I2] Enter point for the next layer is: {}", enter_point);
        }

        // Insert from new_level to layer 1
        for layer_ix in (0..min(insertion_level, self.upper_layers.len())).rev() {
            let mut knn_neighbors = self.search_upper_layers(&self.features[feature_ref], (enter_point, score), EF, layer_ix + 1, &mut visited_neighbors);
            debug!("[I3] Inserting at layer: {}. Numbers of neighbors it will have: {:?}", layer_ix, knn_neighbors.len());
            self.add_node_upper(&mut knn_neighbors, layer_ix);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors.first().unwrap();
            enter_point = self.upper_layers[layer_ix][*nearest_neighbor_index].next_node; 
            score = *nearest_neighbor_distance;

            debug!("[I4] Enter point for the next layer is: {}", enter_point);
        }

        // Insert on layer 0
        let mut knn_neighbors = self.search_layer_zero(&self.features[feature_ref], (enter_point, score), EF, &mut visited_neighbors);
        self.add_node_zero(&mut knn_neighbors);

        // Create new layers up to new_level
        while self.upper_layers.len() < insertion_level {
            let node = Node {
                next_node: if self.upper_layers.len() != 0 {self.upper_layers.last().unwrap().len() - 1 } else { self.zero_layer.len() - 1 },
                feature_index: self.features.len() - 1,
                neighbors: [!0; M],
                neighbor_distances: [!0; M],
                neighbor_count: 0
            };
            self.upper_layers.push(vec![node]);
        }

        //self.print_layers();

        return feature_ref;
    }

    pub fn initialize(&mut self, insertion_level: usize) {
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
            &self.features[feature_idx],
            &self.features[target_feature_ref],
        );
        
        (node_idx, distance)
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

        debug!("[AN] Number of new neighbors: {}", n_neighbors);

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

        debug!("[A0] Node added successfully. Connecting neighbors... | Feature index was: {}", self.features.len() - 1);
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
                let slot = neighbor_node.neighbor_count;
                neighbor_node.neighbors[slot] = new_node_index;
                neighbor_node.neighbor_distances[slot] = new_distance;
                neighbor_node.neighbor_count += 1;
            } else {
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

    fn search_upper_layers(&self, feature: &ID, (enter_point, score): (usize, u32), ef: usize, layer_idx: usize, visited_neighbors: &mut HashSet<usize>) -> Vec<(usize, u32)> {
        let mut candidates: Vec<usize> = vec![enter_point];
        let mut currently_found_nearest_neighbors: Vec<(usize, u32)> = vec![(enter_point, score)];

        while let Some(candidate) = candidates.pop() {
            debug!("[S1] Exploring neighbors from candidate node {} at layer 0.", candidate);
            let neighbors = self.upper_layers[layer_idx - 1][candidate].active_neighbors();

            for neighbor in neighbors {
                let neighbor_feature_index: usize = self.upper_layers[layer_idx - 1][*neighbor].feature_index;
                debug!("    [S2] Exploring neighbor: {:?}", neighbor);
                
                if visited_neighbors.insert(neighbor_feature_index) {
                    let score = D::calculate_distance(
                        &self.features[neighbor_feature_index],
                        feature,
                    );

                    let pos = currently_found_nearest_neighbors.partition_point(|n| n.1 <= score);
                    if pos != ef {
                       debug!("     [S3] CFNN Updated! New neighbor: {:?} at pos {}. Distance is: {:?}", *neighbor, pos, score);
                        if currently_found_nearest_neighbors.len() == ef {
                            currently_found_nearest_neighbors.pop();
                        }
                        currently_found_nearest_neighbors.insert(pos, (*neighbor, score));
                        candidates.push(*neighbor);
                    }
                } 
                
            }
        }

        currently_found_nearest_neighbors

    }

    #[inline(always)]
    fn search_layer_zero(&self, feature: &ID, (enter_point, score): (usize, u32), ef: usize, visited_neighbors: &mut HashSet<usize>) -> Vec<(usize, u32)> {
        let mut candidates: Vec<usize> = vec![enter_point];
        let mut currently_found_nearest_neighbors: Vec<(usize, u32)> = vec![(enter_point, score)];

        while let Some(candidate) = candidates.pop() {
            debug!("[S1] Exploring neighbors from candidate node {} at layer 0.", candidate);
            let neighbors = self.zero_layer[candidate].active_neighbors();

            for neighbor in neighbors {
                let neighbor_feature_index: usize = *neighbor;
                debug!("    [S2] Exploring neighbor: {:?}", neighbor);

                
                if visited_neighbors.insert(neighbor_feature_index) {
                    let score = D::calculate_distance(
                        &self.features[neighbor_feature_index],
                        feature,
                    );

                    let pos = currently_found_nearest_neighbors.partition_point(|n| n.1 <= score);
                    if pos != ef {
                        debug!("    [S3] CFNN Updated! New neighbor: {:?} at pos {}. Distance is: {:?}", *neighbor, pos, score);
                        if currently_found_nearest_neighbors.len() == ef {
                            currently_found_nearest_neighbors.pop();
                        }
                        currently_found_nearest_neighbors.insert(pos, (*neighbor, score));
                        candidates.push(*neighbor);
                    }
                } 
                
            }
        }
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

    /// Performs k-nearest neighbor search in the HNSW structure.
    ///
    /// # Parameters
    /// * `query_id` - The query ID to search for
    /// * `ef` - Size of the dynamic candidate list (exploration factor)
    /// * `k` - Number of nearest neighbors to return
    ///
    /// # Returns
    /// * `Vec<(u32, usize, &ID)>` - Tuples of (distance, index, ID reference)
    pub fn knn_search(&self, query_id: &ID, ef: usize, k: usize) -> Vec<(u32, usize, &ID)> {
        let mut visited_neighbors: HashSet<usize> = HashSet::new();
        
        let (mut enter_point, mut score) = if let Some(top_layer) = self.upper_layers.last() {
            let entry_id_index = top_layer[0].feature_index;
            let initial_distance = D::calculate_distance(&self.features[entry_id_index], query_id);
            (0, initial_distance)
        } else {
            let entry_id_index = self.zero_layer[0].feature_index;
            let initial_distance = D::calculate_distance(&self.features[entry_id_index], query_id);
            (0, initial_distance)
        };

        // Descend through upper layers to layer 0
        for layer_ix in (0..self.upper_layers.len()).rev() {
            let knn_neighbors = self.search_upper_layers(query_id, (enter_point, score), 1,layer_ix + 1, &mut visited_neighbors);
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors[0];
            enter_point = self.upper_layers[layer_ix][nearest_neighbor_index].next_node;
            score = nearest_neighbor_distance;
        }

        // Search at layer 0
        let knn_neighbors = self.search_layer_zero(query_id, (enter_point, score), ef, &mut visited_neighbors);

        knn_neighbors
            .into_iter()
            .map(|(index, distance)| (distance, index, &self.features[index]))
            .collect()
    }


    pub fn get_neighbors_node(&self, index: usize) -> Vec<(u32, usize, &ID)> {
        let mut results: Vec<(u32, usize, &ID)> = vec![];
        results.push((0, index, &self.features[self.zero_layer[index].feature_index]));

        let node = &self.zero_layer[index];
        let neigbors = node.active_neighbors();

        for neighbor_index in neigbors {
            let score = D::calculate_distance(&self.features[*neighbor_index], &self.features[node.feature_index]);
            results.push((score, *neighbor_index, &self.features[*neighbor_index]));
        }

        results
        
    }
 
}

