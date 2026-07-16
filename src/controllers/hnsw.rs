use crate::datalayer::algorithms::DistanceAlgorithm;
use crate::datalayer::nodes::HnswNode;
use core::cmp::Reverse;
use core::cmp::min;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::cmp;
use std::collections::BinaryHeap;
use std::collections::HashSet;
use tracing::debug;

fn default_rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(bound(
    serialize = "ID: serde::Serialize, D: DistanceAlgorithm<ID> + serde::Serialize",
    deserialize = "ID: serde::Deserialize<'de>, D: DistanceAlgorithm<ID> + serde::Deserialize<'de>"
))]
pub struct Hnsw<
    D,
    ID,
    const M: usize,
    const M0: usize,
    const EF: usize = 400,
    const HEURISTIC: bool = false,
> where
    D: DistanceAlgorithm<ID> + Default,
{
    features: Vec<ID>,
    upper_layers: Vec<Vec<HnswNode<M>>>,
    zero_layer: Vec<HnswNode<M0>>,
    #[serde(skip, default = "default_rng")]
    prng: StdRng,
    distance: D,
}

impl<D, ID, const M: usize, const M0: usize, const EF: usize, const HEURISTIC: bool>
    Hnsw<D, ID, M, M0, EF, HEURISTIC>
where
    D: DistanceAlgorithm<ID> + Default,
{
    pub fn new() -> Self {
        Self {
            features: vec![],
            upper_layers: vec![],
            zero_layer: vec![],
            prng: StdRng::seed_from_u64(42),
            distance: D::default(),
        }
    }

    pub fn insert(&mut self, feature: ID) -> usize {
        // TODO: Change return
        let insertion_level = self.random_level();
        debug!(
            "[I0] Inserting new feature. New level to insert is: {}",
            insertion_level
        );
        let feature_ref = self.features.len();
        self.features.push(feature);

        // The data structure is empty
        if self.zero_layer.is_empty() {
            self.initialize(insertion_level);
            return 0;
        }

        let mut visited_neighbors: HashSet<usize> = HashSet::new(); // We use feature indexes here
        let (mut enter_point, mut score) =
            self.get_enter_point(feature_ref, &mut visited_neighbors);

        // Descend to the first insertion level
        for layer_ix in (insertion_level..self.upper_layers.len()).rev() {
            debug!(
                "[I1] Descending to the first insertion level. Current level is: {}",
                layer_ix
            );
            let knn_neighbors: Vec<(usize, u32)> = self.search_upper_layers(
                &self.features[feature_ref],
                (enter_point, score),
                1,
                layer_ix + 1,
                &mut visited_neighbors,
            );
            let (nearest_neighbor_index, nearest_neighbor_distance) =
                knn_neighbors.first().unwrap();
            enter_point = self.upper_layers[layer_ix][*nearest_neighbor_index].next_node as usize; // From the nearest neighbor found, we go to the same node on the next layer
            score = *nearest_neighbor_distance;

            debug!("[I2] Enter point for the next layer is: {}", enter_point);
        }

        // Insert from new_level to layer 1
        for layer_ix in (0..min(insertion_level, self.upper_layers.len())).rev() {
            let mut knn_neighbors = self.search_upper_layers(
                &self.features[feature_ref],
                (enter_point, score),
                EF,
                layer_ix + 1,
                &mut visited_neighbors,
            );
            debug!(
                "[I3] Inserting at layer: {}. Numbers of neighbors it will have: {:?}",
                layer_ix,
                knn_neighbors.len()
            );
            if HEURISTIC {
                self.add_node_heuristic(&mut knn_neighbors, Some(layer_ix));
            } else {
                self.add_node(&mut knn_neighbors, Some(layer_ix));
            }
            let (nearest_neighbor_index, nearest_neighbor_distance) =
                knn_neighbors.first().unwrap();
            enter_point = self.upper_layers[layer_ix][*nearest_neighbor_index].next_node as usize;
            score = *nearest_neighbor_distance;

            debug!("[I4] Enter point for the next layer is: {}", enter_point);
        }

        // Insert on layer 0
        let mut knn_neighbors = self.search_layer_zero(
            &self.features[feature_ref],
            (enter_point, score),
            EF,
            &mut visited_neighbors,
        );
        if HEURISTIC {
            self.add_node_heuristic(&mut knn_neighbors, None);
        } else {
            self.add_node(&mut knn_neighbors, None);
        }

        // Create new layers up to new_level
        while self.upper_layers.len() < insertion_level {
            let node = HnswNode {
                next_node: if self.upper_layers.len() != 0 {
                    self.upper_layers.last().unwrap().len() as u32 - 1
                } else {
                    self.zero_layer.len() as u32 - 1
                },
                feature_index: (self.features.len() - 1) as u32,
                neighbors: [!0; M],
                neighbor_distances: [!0; M],
                neighbor_count: 0,
            };
            self.upper_layers.push(vec![node]);
        }

        //self.print_layers();

        return feature_ref;
    }

    pub fn initialize(&mut self, insertion_level: usize) {
        self.zero_layer.push(HnswNode::new_empty(0, 0));

        // Add the node in higher layers (in case needed) (LAYER 1+)
        while self.upper_layers.len() < insertion_level {
            self.upper_layers.push(vec![HnswNode::new_empty(0, 0)]);
        }
    }

    #[inline]
    fn get_enter_point(
        &self,
        target_feature_ref: usize,
        visited: &mut HashSet<usize>,
    ) -> (usize, u32) {
        let (node_idx, feature_idx) = if self.upper_layers.is_empty() {
            (0, 0)
        } else {
            let entry_feature_idx = self.upper_layers.last().unwrap()[0].feature_index as usize;
            (0, entry_feature_idx)
        };

        visited.insert(feature_idx);
        let distance = self.distance.calculate_distance(
            &self.features[feature_idx],
            &self.features[target_feature_ref],
        );

        (node_idx, distance)
    }

    #[inline]
    fn add_node(&mut self, new_neighbors: &mut Vec<(usize, u32)>, layer_idx: Option<usize>) {
        match layer_idx {
            Some(idx) => {
                // Upper layer insertion
                let new_node_index = self.upper_layers[idx].len() as u32;
                let mut neighbors = [!0; M];
                let mut neighbor_distances = [!0; M];
                new_neighbors.truncate(M);
                let n_neighbors = cmp::min(new_neighbors.len(), M);

                for i in 0..n_neighbors {
                    neighbors[i] = new_neighbors[i].0 as u32;
                    neighbor_distances[i] = new_neighbors[i].1;
                }

                let new_node = HnswNode {
                    feature_index: (self.features.len() - 1) as u32,
                    next_node: if idx == 0 {
                        self.zero_layer.len() as u32
                    } else {
                        self.upper_layers[idx - 1].len() as u32
                    },
                    neighbors,
                    neighbor_distances,
                    neighbor_count: n_neighbors as u16,
                };

                self.upper_layers[idx].push(new_node);
                debug!("[AN] Number of new neighbors: {}", n_neighbors);
                self.connect_neighbors(new_node_index, new_neighbors, Some(idx));
            }
            None => {
                // Zero layer insetion
                let new_node_index = self.zero_layer.len() as u32;
                let mut neighbors = [!0; M0];
                let mut neighbor_distances = [!0; M0];
                new_neighbors.truncate(M0);
                let n_neighbors = cmp::min(new_neighbors.len(), M0);

                for i in 0..n_neighbors {
                    neighbors[i] = new_neighbors[i].0 as u32;
                    neighbor_distances[i] = new_neighbors[i].1;
                }

                self.zero_layer.push(HnswNode {
                    feature_index: (self.features.len() - 1) as u32,
                    next_node: 0,
                    neighbors,
                    neighbor_distances,
                    neighbor_count: n_neighbors as u16,
                });

                debug!(
                    "[A0] Node added successfully. Now connecting neighbors... | Feature index was: {}",
                    self.features.len() - 1
                );
                self.connect_neighbors(new_node_index, new_neighbors, None);
            }
        }
    }

    /// Selects up to `max_neighbors` (heuristic).
    /// Each candidate is (node_index, feature_index, distance).
    /// Returns (node_index, distance) pairs for the selected neighbors.
    #[inline]
    fn select_neighbors_heuristic(
        &self,
        candidates: &[(u32, usize, u32)],
        max_neighbors: usize,
    ) -> Vec<(u32, u32)> {
        let mut new_confirmed_neighbors: Vec<(u32, usize, u32)> = Vec::with_capacity(max_neighbors);

        // Iterate each neighbor to see who's a better link. Our new node, or some of the new neighbors?
        for &(cand_node, candidate_feature_idx, candidate_distance) in candidates {
            // Greedy set cover, this is, we only check candidate against ALREADY-SELECTED (confirmed) neighbors
            let is_new_node_best_candidate =
                new_confirmed_neighbors
                    .iter()
                    .all(|&(_, selected_feature_idx, _)| {
                        self.distance.calculate_distance(
                            &self.features[candidate_feature_idx],
                            &self.features[selected_feature_idx],
                        ) >= candidate_distance
                    });

            if is_new_node_best_candidate {
                // The node we are inserting is the best link
                new_confirmed_neighbors.push((
                    cand_node,
                    candidate_feature_idx,
                    candidate_distance,
                ));
                if new_confirmed_neighbors.len() == max_neighbors {
                    break;
                }
            } // Else, some of the other neighbors is better than us (and will be linked to it later)!
        }

        new_confirmed_neighbors
            .iter()
            .map(|&(node, _, dist)| (node, dist))
            .collect()
    }

    #[inline]
    fn add_node_heuristic(
        &mut self,
        new_neighbors: &mut Vec<(usize, u32)>,
        layer_idx: Option<usize>,
    ) {
        let new_feature_idx = self.features.len() - 1;

        let new_node_index = match layer_idx {
            Some(idx) => self.upper_layers[idx].len() as u32,
            None => self.zero_layer.len() as u32,
        };

        let candidates: Vec<(u32, usize, u32)> = new_neighbors // (neighbor_idx, feature_idx, distance)
            .iter()
            .map(|&(node, dist)| {
                let feature = match layer_idx {
                    Some(idx) => self.upper_layers[idx][node].feature_index as usize,
                    None => self.zero_layer[node].feature_index as usize,
                };
                (node as u32, feature, dist)
            })
            .collect();

        let max_neighbors = if layer_idx.is_some() { M } else { M0 };
        let selected = self.select_neighbors_heuristic(&candidates, max_neighbors);

        // Node construction and insertion differ by the layer we are (HnswNode<M> vs HnswNode<M0>)
        match layer_idx {
            Some(idx) => {
                let mut neighbors = [!0u32; M];
                let mut neighbor_distances = [!0u32; M];
                for (i, &(node, dist)) in selected.iter().enumerate() {
                    neighbors[i] = node;
                    neighbor_distances[i] = dist;
                }

                let next_node = if idx == 0 {
                    self.zero_layer.len() as u32
                } else {
                    self.upper_layers[idx - 1].len() as u32
                };

                self.upper_layers[idx].push(HnswNode {
                    feature_index: new_feature_idx as u32,
                    next_node,
                    neighbors,
                    neighbor_distances,
                    neighbor_count: selected.len() as u16,
                });

                let selected_as_usize: Vec<(usize, u32)> =
                    selected.iter().map(|&(n, d)| (n as usize, d)).collect();
                self.connect_neighbors_heuristic(new_node_index, &selected_as_usize, Some(idx));
            }
            None => {
                let mut neighbors = [!0u32; M0];
                let mut neighbor_distances = [!0u32; M0];
                for (i, &(node, dist)) in selected.iter().enumerate() {
                    neighbors[i] = node;
                    neighbor_distances[i] = dist;
                }

                self.zero_layer.push(HnswNode {
                    feature_index: new_feature_idx as u32,
                    next_node: 0,
                    neighbors,
                    neighbor_distances,
                    neighbor_count: selected.len() as u16,
                });

                let selected_as_usize: Vec<(usize, u32)> =
                    selected.iter().map(|&(n, d)| (n as usize, d)).collect();
                self.connect_neighbors_heuristic(new_node_index, &selected_as_usize, None);
            }
        }
    }

    // Connects a single node to its new selected neighbors.
    // If the neighbor list is full, select only better candidates.
    #[inline]
    fn connect_neighbors(
        &mut self,
        new_node_index: u32,
        new_neighbors: &[(usize, u32)],
        layer_idx: Option<usize>,
    ) {
        match layer_idx {
            Some(idx) => {
                for &(neighbor_idx, new_distance) in new_neighbors {
                    let neighbor_node = &mut self.upper_layers[idx][neighbor_idx];

                    if (neighbor_node.neighbor_count as usize) < M {
                        // We have space on the neighbor's list. Just add it.
                        let slot = neighbor_node.neighbor_count as usize;
                        neighbor_node.neighbors[slot] = new_node_index;
                        neighbor_node.neighbor_distances[slot] = new_distance;
                        neighbor_node.neighbor_count += 1;
                    } else {
                        // Neighbor list is full, replace it with the farest one
                        let (worst_ix, worst_distance) = neighbor_node
                            .active_distances()
                            .iter()
                            .enumerate()
                            .max_by_key(|&(_, &distance)| distance)
                            .unwrap();

                        if new_distance < *worst_distance {
                            // Bidirecionally connect the nodes ONLY if we have space
                            neighbor_node.neighbors[worst_ix] = new_node_index;
                            neighbor_node.neighbor_distances[worst_ix] = new_distance;
                        }
                    }
                }
            }
            None => {
                for &(neighbor_idx, new_distance) in new_neighbors {
                    let neighbor_node = &mut self.zero_layer[neighbor_idx];

                    if (neighbor_node.neighbor_count as usize) < M0 {
                        let slot = neighbor_node.neighbor_count as usize;
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
        }
    }

    #[inline]
    fn connect_neighbors_heuristic(
        &mut self,
        new_node_index: u32,
        new_neighbors: &[(usize, u32)],
        layer_idx: Option<usize>,
    ) {
        let new_node_feature_idx = self.features.len() - 1;
        let max_neighbors = if layer_idx.is_some() { M } else { M0 };

        for &(neighbor_idx, new_distance) in new_neighbors {
            let neighbor_count = match layer_idx {
                Some(idx) => self.upper_layers[idx][neighbor_idx].neighbor_count as usize,
                None => self.zero_layer[neighbor_idx].neighbor_count as usize,
            };

            let mut candidates: Vec<(u32, usize, u32)> = Vec::with_capacity(max_neighbors + 1);

            // Retrieve all new neighbors with its distances to the new node to later heuristic compare.
            for i in 0..neighbor_count {
                let (node, distance_to_new_node) = match layer_idx {
                    Some(idx) => (
                        self.upper_layers[idx][neighbor_idx].neighbors[i],
                        self.upper_layers[idx][neighbor_idx].neighbor_distances[i],
                    ),
                    None => (
                        self.zero_layer[neighbor_idx].neighbors[i],
                        self.zero_layer[neighbor_idx].neighbor_distances[i],
                    ),
                };

                let feature: usize = match layer_idx {
                    Some(idx) => self.upper_layers[idx][node as usize].feature_index as usize,
                    None => self.zero_layer[node as usize].feature_index as usize,
                };

                candidates.push((node, feature, distance_to_new_node));
            }
            // We also include the new node.
            candidates.push((new_node_index, new_node_feature_idx, new_distance));
            // Required: select_neighbors_heuristic() needs candidates in ascending distance order.
            candidates.sort_by_key(|&(_, _, d)| d);

            // Now, evaluate each new neighbor's neighbor list VS. the new node, BUT WITH HEURISTIC!
            // Second time we call here and some ops may be redundant, could this be cached somehow or i'm just rambling?
            let selected = self.select_neighbors_heuristic(&candidates, max_neighbors);

            // Replace the old neighbors list with the new one
            match layer_idx {
                Some(idx) => {
                    let neighbor_node = &mut self.upper_layers[idx][neighbor_idx];
                    for (i, &(node, dist)) in selected.iter().enumerate() {
                        neighbor_node.neighbors[i] = node;
                        neighbor_node.neighbor_distances[i] = dist;
                    }
                    neighbor_node.neighbor_count = selected.len() as u16;
                }
                None => {
                    let neighbor_node = &mut self.zero_layer[neighbor_idx];
                    for (i, &(node, dist)) in selected.iter().enumerate() {
                        neighbor_node.neighbors[i] = node;
                        neighbor_node.neighbor_distances[i] = dist;
                    }
                    neighbor_node.neighbor_count = selected.len() as u16;
                }
            }
        }
    }

    #[inline]
    fn search_upper_layers(
        &self,
        query_feature: &ID,
        (enter_point, score): (usize, u32),
        ef: usize,
        layer_idx: usize,
        visited_neighbors: &mut HashSet<usize>,
    ) -> Vec<(usize, u32)> {
        let mut candidates: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::with_capacity(ef); // Min heap
        candidates.push(Reverse((score, enter_point)));
        let mut currently_found_nearest_neighbors: BinaryHeap<(u32, usize)> =
            BinaryHeap::with_capacity(ef + 1); // Max heap
        currently_found_nearest_neighbors.push((score, enter_point));

        while let Some(Reverse((candidate_dist, candidate))) = candidates.pop() {
            // Early stop: if the closest remaining candidate is already farther than
            // our current worst neighbor, no node reachable from here can improve the result.
            if currently_found_nearest_neighbors.len() >= ef
                && candidate_dist > currently_found_nearest_neighbors.peek().unwrap().0
            {
                break;
            }
            debug!(
                "[S1] Exploring neighbors from candidate node {} at layer 0.",
                candidate
            );
            // Get neighbors from the candidate
            let neighbors = self.upper_layers[layer_idx - 1][candidate].active_neighbors();

            for neighbor in neighbors {
                let neighbor_feature_index: usize = // Get the current neighbor's feature index
                    self.upper_layers[layer_idx - 1][*neighbor as usize].feature_index as usize;
                debug!("    [S2] Exploring neighbor: {:?}", neighbor);

                if visited_neighbors.insert(neighbor_feature_index) {
                    // We have NOT visited the current neighbor
                    let score = self
                        .distance
                        .calculate_distance(&self.features[neighbor_feature_index], query_feature);

                    if currently_found_nearest_neighbors.len() < ef
                        || score < currently_found_nearest_neighbors.peek().unwrap().0
                    {
                        candidates.push(Reverse((score, *neighbor as usize)));
                        currently_found_nearest_neighbors.push((score, *neighbor as usize));
                        if currently_found_nearest_neighbors.len() > ef {
                            currently_found_nearest_neighbors.pop(); // evict the farthest
                        }
                    }
                }
            }
        }

        currently_found_nearest_neighbors
            .into_sorted_vec()
            .into_iter()
            .map(|(dist, idx)| (idx, dist))
            .collect()
    }

    #[inline]
    fn search_layer_zero(
        &self,
        query_feature: &ID,
        (enter_point, score): (usize, u32),
        ef: usize,
        visited_neighbors: &mut HashSet<usize>,
    ) -> Vec<(usize, u32)> {
        let mut candidates: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::with_capacity(ef); // Min heap
        candidates.push(Reverse((score, enter_point)));
        let mut currently_found_nearest_neighbors: BinaryHeap<(u32, usize)> =
            BinaryHeap::with_capacity(ef + 1); // Max heap
        currently_found_nearest_neighbors.push((score, enter_point));

        while let Some(Reverse((candidate_dist, candidate))) = candidates.pop() {
            // Early stop: if the closest remaining candidate is already farther than
            // our current worst neighbor, no node reachable from here can improve the result.
            if currently_found_nearest_neighbors.len() >= ef
                && candidate_dist > currently_found_nearest_neighbors.peek().unwrap().0
            {
                break;
            }

            debug!(
                "[S1] Exploring neighbors from candidate node {} at layer 0.",
                candidate
            );
            let neighbors = self.zero_layer[candidate].active_neighbors();

            for neighbor in neighbors {
                let neighbor_feature_index: usize = *neighbor as usize;
                debug!("    [S2] Exploring neighbor: {:?}", neighbor);

                if visited_neighbors.insert(neighbor_feature_index) {
                    let score = self
                        .distance
                        .calculate_distance(&self.features[neighbor_feature_index], query_feature);

                    // Admit the neighbor if there is still space, or if it beats
                    // the current worst keeper.
                    if currently_found_nearest_neighbors.len() < ef
                        || score < currently_found_nearest_neighbors.peek().unwrap().0
                    {
                        debug!(
                            "    [S3] CFNN Updated! New neighbor: {:?}. Distance is: {:?}",
                            *neighbor, score
                        );
                        candidates.push(Reverse((score, neighbor_feature_index)));
                        currently_found_nearest_neighbors.push((score, neighbor_feature_index));
                        if currently_found_nearest_neighbors.len() > ef {
                            currently_found_nearest_neighbors.pop(); // evict the farthest
                        }
                    }
                }
            }
        }

        // Return ascending by distance, as (node_index, distance) — the contract
        // callers rely on (`take(k)`, and `[0]` == nearest).
        currently_found_nearest_neighbors
            .into_sorted_vec()
            .into_iter()
            .map(|(dist, idx)| (idx, dist))
            .collect()
    }

    fn random_level(&mut self) -> usize {
        let uniform: f64 = self.prng.next_u64() as f64 / core::u64::MAX as f64;
        (-libm::log(uniform) * libm::log(M as f64).recip()) as usize
    }

    // Debugging function to print HNSW layers (all nodes and their connections)
    pub fn print_layers(&mut self) {
        println!("======================== LAYER 0 ========================");
        for node in 0..self.zero_layer.len() {
            print!("Node {} | Neighbors: ", node);
            let mut act_neighbors: Vec<u32> = vec![];
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
            println!(
                "======================== LAYER {} ========================",
                layer_idx + 1
            );
            for node in 0..self.upper_layers[layer_idx].len() {
                print!("Node {} | Neighbors: ", node);
                let mut act_neighbors: Vec<u32> = vec![];
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
    pub fn knn_search(&self, query_id: &ID, k: usize, ef: usize) -> Vec<(u32, usize, &ID)> {
        let mut visited_neighbors: HashSet<usize> = HashSet::new();

        let (mut enter_point, mut score) = if let Some(top_layer) = self.upper_layers.last() {
            let entry_id_index = top_layer[0].feature_index as usize;
            let initial_distance = self
                .distance
                .calculate_distance(&self.features[entry_id_index], query_id);
            (0, initial_distance)
        } else {
            let entry_id_index = self.zero_layer[0].feature_index as usize;
            let initial_distance = self
                .distance
                .calculate_distance(&self.features[entry_id_index], query_id);
            (0, initial_distance)
        };

        // Descend through upper layers to layer 0
        for layer_ix in (0..self.upper_layers.len()).rev() {
            let knn_neighbors = self.search_upper_layers(
                query_id,
                (enter_point, score),
                1,
                layer_ix + 1,
                &mut visited_neighbors,
            );
            let (nearest_neighbor_index, nearest_neighbor_distance) = knn_neighbors[0];
            enter_point = self.upper_layers[layer_ix][nearest_neighbor_index].next_node as usize;
            score = nearest_neighbor_distance;
        }

        // Search at layer 0
        let knn_neighbors =
            self.search_layer_zero(query_id, (enter_point, score), ef, &mut visited_neighbors);

        knn_neighbors
            .into_iter()
            .take(k)
            .map(|(index, distance)| (distance, index, &self.features[index]))
            .collect()
    }

    pub fn get_neighbors_node(&self, index: usize) -> Vec<(u32, usize, &ID)> {
        let mut results: Vec<(u32, usize, &ID)> = vec![];
        results.push((
            0,
            index,
            &self.features[self.zero_layer[index].feature_index as usize],
        ));

        let node = &self.zero_layer[index];
        let neigbors = node.active_neighbors();

        for neighbor_index in neigbors {
            let score = self.distance.calculate_distance(
                &self.features[*neighbor_index as usize],
                &self.features[node.feature_index as usize],
            );
            results.push((
                score,
                *neighbor_index as usize,
                &self.features[*neighbor_index as usize],
            ));
        }

        results
    }

    pub fn draw_model(&self) -> Vec<(Vec<usize>, Vec<(String, String, f32)>)> {
        let mut layer_gexfs = Vec::new();

        let nodes_edges_layer0 = self.get_nodes_with_edges(0, &self.zero_layer);
        layer_gexfs.push(nodes_edges_layer0);

        for (idx, layer) in self.upper_layers.iter().enumerate() {
            let nodes_edges_layer = self.get_nodes_with_edges(idx + 1, layer);
            layer_gexfs.push(nodes_edges_layer);
        }

        layer_gexfs
    }

    /// Creates a GEXF structure for a single layer with nodes and edges
    fn get_nodes_with_edges<const N: usize>(
        &self,
        layer_idx: usize,
        nodes: &[HnswNode<N>],
    ) -> (Vec<usize>, Vec<(String, String, f32)>) {
        let mut seen_edges: HashSet<(usize, usize)> = HashSet::new();
        let mut draw_nodes: Vec<usize> = Vec::new();
        let mut draw_edges: Vec<(String, String, f32)> = Vec::new();

        // Add all nodes
        for n in nodes {
            draw_nodes.push(n.feature_index as usize);
        }

        // Add edges between nodes
        for node in nodes {
            for (nb_i, &neighbor_idx) in node.active_neighbors().iter().enumerate() {
                let source_node_idx = node.feature_index as usize;

                let target_node_idx = if layer_idx == 0 {
                    self.zero_layer[neighbor_idx as usize].feature_index as usize
                } else {
                    self.upper_layers[layer_idx - 1][neighbor_idx as usize].feature_index as usize
                };

                // Avoid duplicate edges in bidirectional cases
                let edge_pair = if source_node_idx < target_node_idx {
                    (source_node_idx, target_node_idx)
                } else {
                    (target_node_idx, source_node_idx)
                };

                if seen_edges.insert(edge_pair) {
                    let dist = node.neighbor_distances[nb_i];
                    draw_edges.push((
                        source_node_idx.to_string(),
                        target_node_idx.to_string(),
                        dist as f32,
                    ));
                }
            }
        }

        (draw_nodes, draw_edges)
    }
}
