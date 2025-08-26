//! Navigation through semantic spacetime

use crate::{
    SemanticCoords, SemanticPosition, SemanticDistance,
    SemanticLink, SemanticRelation, RelationType,
    Story, StoryPath, SemanticResult,
};
use uuid::Uuid;
use std::collections::{HashMap, HashSet, VecDeque};
use petgraph::{
    graph::{DiGraph, NodeIndex},
    algo::{dijkstra, all_simple_paths},
    Direction,
};

/// Navigator for traversing semantic spacetime
pub struct SemanticNavigator {
    /// Graph of semantic agents and their relations
    graph: DiGraph<AgentNode, SemanticLink>,
    
    /// Mapping from agent ID to graph node
    node_map: HashMap<Uuid, NodeIndex>,
    
    /// Current position in semantic space
    current_position: Option<SemanticPosition>,
    
    /// History of positions visited
    path_history: Vec<SemanticPosition>,
    
    /// Distance metric to use
    distance_metric: SemanticDistance,
}

impl SemanticNavigator {
    /// Create a new navigator
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            current_position: None,
            path_history: Vec::new(),
            distance_metric: SemanticDistance::Euclidean,
        }
    }
    
    /// Add an agent to the semantic space
    pub fn add_agent(&mut self, agent_id: Uuid, position: SemanticCoords, name: String) {
        let node = AgentNode {
            id: agent_id,
            position,
            name,
            visited_count: 0,
        };
        
        let idx = self.graph.add_node(node);
        self.node_map.insert(agent_id, idx);
    }
    
    /// Connect two agents with a semantic relation
    pub fn connect_agents(
        &mut self,
        from: Uuid,
        to: Uuid,
        relation: SemanticRelation,
    ) -> SemanticResult<()> {
        let from_idx = self.node_map.get(&from)
            .ok_or("Source agent not found")?;
        let to_idx = self.node_map.get(&to)
            .ok_or("Target agent not found")?;
        
        let link = SemanticLink::new(from, to, relation);
        self.graph.add_edge(*from_idx, *to_idx, link);
        
        Ok(())
    }
    
    /// Find nearest agents by semantic distance
    pub fn find_nearest(&self, position: SemanticCoords, limit: usize) -> Vec<(Uuid, f64)> {
        let mut distances: Vec<(Uuid, f64)> = self.graph
            .node_weights()
            .map(|node| {
                let distance = self.distance_metric.calculate(&position, &node.position);
                (node.id, distance)
            })
            .collect();
        
        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        distances.truncate(limit);
        distances
    }
    
    /// Find agents with high affinity for a given intent
    pub fn find_by_affinity(
        &self,
        intent_coords: SemanticCoords,
        threshold: f64,
    ) -> Vec<Uuid> {
        self.graph
            .node_weights()
            .filter(|node| {
                let similarity = node.position.similarity_to(&intent_coords);
                similarity >= threshold
            })
            .map(|node| node.id)
            .collect()
    }
    
    /// Navigate to a target position
    pub fn navigate_to(&mut self, target: SemanticCoords) -> StoryPath {
        let mut path = StoryPath::new();
        
        if let Some(current) = &self.current_position {
            path.add_position(current.coords);
        }
        
        path.add_position(target);
        
        // Update current position
        self.current_position = Some(SemanticPosition::new(
            target,
            "navigation".to_string(),
            vec![],
        ));
        
        // Add to history
        if let Some(pos) = &self.current_position {
            self.path_history.push(pos.clone());
        }
        
        path
    }
    
    /// Find shortest path between two agents
    pub fn find_shortest_path(&self, from: Uuid, to: Uuid) -> Option<Vec<Uuid>> {
        let from_idx = self.node_map.get(&from)?;
        let to_idx = self.node_map.get(&to)?;
        
        let result = dijkstra(
            &self.graph,
            *from_idx,
            Some(*to_idx),
            |e| e.weight().effective_strength(),
        );
        
        if result.contains_key(to_idx) {
            // Reconstruct path
            let mut path = vec![to];
            let mut current = *to_idx;
            
            while current != *from_idx {
                // Find predecessor
                for edge in self.graph.edges_directed(current, Direction::Incoming) {
                    let source = edge.source();
                    if result.contains_key(&source) {
                        if let Some(node) = self.graph.node_weight(source) {
                            path.push(node.id);
                            current = source;
                            break;
                        }
                    }
                }
            }
            
            path.reverse();
            Some(path)
        } else {
            None
        }
    }
    
    /// Find all paths between agents
    pub fn find_all_paths(&self, from: Uuid, to: Uuid, max_length: usize) -> Vec<Vec<Uuid>> {
        let from_idx = match self.node_map.get(&from) {
            Some(idx) => *idx,
            None => return vec![],
        };
        
        let to_idx = match self.node_map.get(&to) {
            Some(idx) => *idx,
            None => return vec![],
        };
        
        let paths: Vec<Vec<NodeIndex>> = all_simple_paths(
            &self.graph,
            from_idx,
            to_idx,
            0,
            Some(max_length),
        ).collect();
        
        paths.iter()
            .map(|path| {
                path.iter()
                    .filter_map(|idx| self.graph.node_weight(*idx).map(|n| n.id))
                    .collect()
            })
            .collect()
    }
    
    /// Explore neighborhood around current position
    pub fn explore_neighborhood(&self, radius: f64) -> Vec<AgentNode> {
        if let Some(current) = &self.current_position {
            self.graph
                .node_weights()
                .filter(|node| {
                    let distance = self.distance_metric.calculate(
                        &current.coords,
                        &node.position,
                    );
                    distance <= radius
                })
                .cloned()
                .collect()
        } else {
            vec![]
        }
    }
    
    /// Get semantic clusters (agents that are closely related)
    pub fn find_clusters(&self, min_cluster_size: usize) -> Vec<Vec<Uuid>> {
        let mut clusters: Vec<Vec<Uuid>> = vec![];
        let mut visited: HashSet<Uuid> = HashSet::new();
        
        for node in self.graph.node_weights() {
            if visited.contains(&node.id) {
                continue;
            }
            
            let mut cluster = vec![node.id];
            visited.insert(node.id);
            
            // BFS to find connected components
            let mut queue = VecDeque::new();
            if let Some(idx) = self.node_map.get(&node.id) {
                queue.push_back(*idx);
            }
            
            while let Some(current_idx) = queue.pop_front() {
                for edge in self.graph.edges(current_idx) {
                    let target_idx = edge.target();
                    if let Some(target_node) = self.graph.node_weight(target_idx) {
                        if !visited.contains(&target_node.id) {
                            // Check semantic distance
                            let distance = self.distance_metric.calculate(
                                &node.position,
                                &target_node.position,
                            );
                            
                            if distance < 0.3 {  // Close enough to be in cluster
                                cluster.push(target_node.id);
                                visited.insert(target_node.id);
                                queue.push_back(target_idx);
                            }
                        }
                    }
                }
            }
            
            if cluster.len() >= min_cluster_size {
                clusters.push(cluster);
            }
        }
        
        clusters
    }
    
    /// Get the most traversed paths
    pub fn popular_paths(&self, limit: usize) -> Vec<(Vec<Uuid>, f64)> {
        let mut path_weights: HashMap<Vec<Uuid>, f64> = HashMap::new();
        
        // Collect all edges with their traversal counts
        for edge in self.graph.edge_weights() {
            let path = vec![edge.from, edge.to];
            *path_weights.entry(path).or_insert(0.0) += edge.success_rate * edge.traversal_count as f64;
        }
        
        // Sort by weight
        let mut sorted: Vec<_> = path_weights.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        sorted.truncate(limit);
        sorted
    }
}

/// Node in the semantic graph
#[derive(Debug, Clone)]
pub struct AgentNode {
    /// Agent ID
    pub id: Uuid,
    
    /// Semantic position
    pub position: SemanticCoords,
    
    /// Agent name
    pub name: String,
    
    /// Number of times visited
    pub visited_count: u64,
}

/// Pathfinding algorithm options
#[derive(Debug, Clone, Copy)]
pub enum PathfindingAlgorithm {
    /// Dijkstra's shortest path
    Dijkstra,
    /// A* with semantic heuristic
    AStar,
    /// Find all simple paths
    AllPaths,
    /// Random walk
    RandomWalk,
}

/// Path quality metrics
#[derive(Debug, Clone)]
pub struct PathMetrics {
    /// Total semantic distance
    pub total_distance: f64,
    
    /// Average trust along path
    pub average_trust: f64,
    
    /// Number of hops
    pub hop_count: usize,
    
    /// Semantic coherence (how well the path stays in context)
    pub coherence: f64,
}

impl PathMetrics {
    /// Calculate metrics for a story path
    pub fn from_story_path(path: &StoryPath) -> Self {
        Self {
            total_distance: path.total_distance,
            average_trust: path.average_trust,
            hop_count: path.positions.len(),
            coherence: Self::calculate_coherence(&path.positions),
        }
    }
    
    /// Calculate semantic coherence of positions
    fn calculate_coherence(positions: &[SemanticCoords]) -> f64 {
        if positions.len() < 2 {
            return 1.0;
        }
        
        let mut coherence = 0.0;
        for window in positions.windows(2) {
            coherence += window[0].similarity_to(&window[1]);
        }
        
        coherence / (positions.len() - 1) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relations::common;
    
    #[test]
    fn test_navigator_creation() {
        let mut nav = SemanticNavigator::new();
        let agent_id = Uuid::new_v4();
        let position = SemanticCoords::new(0.5, 0.5, 0.5, 0.5);
        
        nav.add_agent(agent_id, position, "TestAgent".to_string());
        assert_eq!(nav.node_map.len(), 1);
    }
    
    #[test]
    fn test_find_nearest() {
        let mut nav = SemanticNavigator::new();
        
        // Add some agents
        let agent1 = Uuid::new_v4();
        let agent2 = Uuid::new_v4();
        let agent3 = Uuid::new_v4();
        
        nav.add_agent(agent1, SemanticCoords::new(0.0, 0.0, 0.0, 0.0), "Agent1".to_string());
        nav.add_agent(agent2, SemanticCoords::new(0.1, 0.1, 0.1, 0.1), "Agent2".to_string());
        nav.add_agent(agent3, SemanticCoords::new(1.0, 1.0, 1.0, 1.0), "Agent3".to_string());
        
        let nearest = nav.find_nearest(SemanticCoords::new(0.0, 0.0, 0.0, 0.0), 2);
        assert_eq!(nearest.len(), 2);
        assert_eq!(nearest[0].0, agent1); // Closest to origin
    }
}