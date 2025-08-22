//! Intent tree and forest structures for managing hierarchical intents

use crate::{
    types::*, intent::HierarchicalIntent, IntentError, Result
};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Deserialize, Serialize};

/// Relationship between intents in a tree
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentRelation {
    /// Parent-child relationship
    Parent,
    /// Sequential dependency
    DependsOn,
    /// Conflict relationship (cannot execute together)
    Conflicts,
    /// Complementary relationship (better together)
    Complements,
}

/// A tree structure for managing hierarchical intents
pub struct IntentTree {
    /// The graph structure
    graph: DiGraph<IntentNode, IntentRelation>,
    /// Map from intent ID to node index
    node_map: HashMap<IntentId, NodeIndex>,
    /// Root intents (no parents)
    roots: HashSet<NodeIndex>,
}

/// Node in the intent tree
#[derive(Debug, Clone)]
pub struct IntentNode {
    /// The intent
    pub intent: HierarchicalIntent,
    /// Depth in the tree
    pub depth: usize,
    /// Execution order (if determined)
    pub execution_order: Option<usize>,
}

impl IntentTree {
    /// Creates a new intent tree
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            roots: HashSet::new(),
        }
    }
    
    /// Adds an intent to the tree
    pub fn add_intent(&mut self, intent: HierarchicalIntent) -> NodeIndex {
        let intent_id = intent.id();
        let depth = 0; // Will be updated
        
        let node = IntentNode {
            intent,
            depth,
            execution_order: None,
        };
        
        let idx = self.graph.add_node(node);
        self.node_map.insert(intent_id, idx);
        self.roots.insert(idx);
        
        idx
    }
    
    /// Adds a relationship between intents
    pub fn add_relation(
        &mut self,
        from: IntentId,
        to: IntentId,
        relation: IntentRelation,
    ) -> Result<()> {
        let from_idx = self.node_map.get(&from)
            .ok_or_else(|| IntentError::NotFound(from.0))?;
        let to_idx = self.node_map.get(&to)
            .ok_or_else(|| IntentError::NotFound(to.0))?;
        
        self.graph.add_edge(*from_idx, *to_idx, relation);
        
        // Update roots if necessary
        if relation == IntentRelation::Parent {
            self.roots.remove(to_idx);
            self.update_depths(*from_idx)?;
        }
        
        Ok(())
    }
    
    /// Gets all children of an intent
    pub fn get_children(&self, intent_id: IntentId) -> Vec<IntentId> {
        if let Some(&node_idx) = self.node_map.get(&intent_id) {
            self.graph
                .edges_directed(node_idx, Direction::Outgoing)
                .filter(|edge| *edge.weight() == IntentRelation::Parent)
                .map(|edge| self.graph[edge.target()].intent.id())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Gets the parent of an intent
    pub fn get_parent(&self, intent_id: IntentId) -> Option<IntentId> {
        if let Some(&node_idx) = self.node_map.get(&intent_id) {
            self.graph
                .edges_directed(node_idx, Direction::Incoming)
                .find(|edge| *edge.weight() == IntentRelation::Parent)
                .map(|edge| self.graph[edge.source()].intent.id())
        } else {
            None
        }
    }
    
    /// Gets all dependencies of an intent
    pub fn get_dependencies(&self, intent_id: IntentId) -> Vec<IntentId> {
        if let Some(&node_idx) = self.node_map.get(&intent_id) {
            self.graph
                .edges_directed(node_idx, Direction::Incoming)
                .filter(|edge| *edge.weight() == IntentRelation::DependsOn)
                .map(|edge| self.graph[edge.source()].intent.id())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Gets all intents that depend on this one
    pub fn get_dependents(&self, intent_id: IntentId) -> Vec<IntentId> {
        if let Some(&node_idx) = self.node_map.get(&intent_id) {
            self.graph
                .edges_directed(node_idx, Direction::Outgoing)
                .filter(|edge| *edge.weight() == IntentRelation::DependsOn)
                .map(|edge| self.graph[edge.target()].intent.id())
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Finds conflicts for an intent
    pub fn find_conflicts(&self, intent_id: IntentId) -> Vec<IntentId> {
        if let Some(&node_idx) = self.node_map.get(&intent_id) {
            self.graph
                .edges(node_idx)
                .filter(|edge| *edge.weight() == IntentRelation::Conflicts)
                .map(|edge| {
                    let other = if edge.source() == node_idx {
                        edge.target()
                    } else {
                        edge.source()
                    };
                    self.graph[other].intent.id()
                })
                .collect()
        } else {
            Vec::new()
        }
    }
    
    /// Performs a breadth-first traversal
    pub fn bfs_traverse(&self, start: IntentId) -> Vec<IntentId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut result = Vec::new();
        
        if let Some(&start_idx) = self.node_map.get(&start) {
            queue.push_back(start_idx);
            visited.insert(start_idx);
            
            while let Some(current) = queue.pop_front() {
                result.push(self.graph[current].intent.id());
                
                for neighbor in self.graph.neighbors(current) {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        
        result
    }
    
    /// Performs a depth-first traversal
    pub fn dfs_traverse(&self, start: IntentId) -> Vec<IntentId> {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let mut result = Vec::new();
        
        if let Some(&start_idx) = self.node_map.get(&start) {
            stack.push(start_idx);
            
            while let Some(current) = stack.pop() {
                if !visited.contains(&current) {
                    visited.insert(current);
                    result.push(self.graph[current].intent.id());
                    
                    for neighbor in self.graph.neighbors(current) {
                        if !visited.contains(&neighbor) {
                            stack.push(neighbor);
                        }
                    }
                }
            }
        }
        
        result
    }
    
    /// Determines execution order considering dependencies
    pub fn determine_execution_order(&mut self) -> Result<Vec<IntentId>> {
        // Create a dependency-only graph
        let mut dep_graph = DiGraph::new();
        let mut dep_map = HashMap::new();
        
        // Add all nodes
        for (&intent_id, &node_idx) in &self.node_map {
            let dep_idx = dep_graph.add_node(intent_id);
            dep_map.insert(node_idx, dep_idx);
        }
        
        // Add dependency edges
        for edge in self.graph.edge_indices() {
            let (source, target) = self.graph.edge_endpoints(edge).unwrap();
            if let Some(weight) = self.graph.edge_weight(edge) {
                if *weight == IntentRelation::DependsOn {
                    dep_graph.add_edge(dep_map[&source], dep_map[&target], ());
                }
            }
        }
        
        // Topological sort
        let sorted = petgraph::algo::toposort(&dep_graph, None)
            .map_err(|_| IntentError::ValidationFailed("Circular dependencies detected".to_string()))?;
        
        // Update execution order in nodes
        let mut order = Vec::new();
        for (i, &dep_idx) in sorted.iter().enumerate() {
            let intent_id = dep_graph[dep_idx];
            if let Some(&node_idx) = self.node_map.get(&intent_id) {
                self.graph[node_idx].execution_order = Some(i);
            }
            order.push(intent_id);
        }
        
        Ok(order)
    }
    
    /// Gets all root intents
    pub fn get_roots(&self) -> Vec<IntentId> {
        self.roots
            .iter()
            .map(|&idx| self.graph[idx].intent.id())
            .collect()
    }
    
    /// Gets all leaf intents (no children)
    pub fn get_leaves(&self) -> Vec<IntentId> {
        self.graph
            .node_indices()
            .filter(|&idx| {
                self.graph
                    .edges_directed(idx, Direction::Outgoing)
                    .filter(|edge| *edge.weight() == IntentRelation::Parent)
                    .count() == 0
            })
            .map(|idx| self.graph[idx].intent.id())
            .collect()
    }
    
    /// Validates the tree structure
    pub async fn validate(&self) -> Result<()> {
        // Check for circular dependencies
        if petgraph::algo::is_cyclic_directed(&self.graph) {
            // Check if cycles are only from Conflicts relations
            let mut dep_graph = self.graph.clone();
            dep_graph.retain_edges(|graph, edge_idx| {
                if let Some(weight) = graph.edge_weight(edge_idx) {
                    *weight != IntentRelation::Conflicts
                } else {
                    true
                }
            });
            
            if petgraph::algo::is_cyclic_directed(&dep_graph) {
                return Err(IntentError::ValidationFailed(
                    "Circular dependencies detected".to_string()
                ));
            }
        }
        
        // Validate each intent
        for node in self.graph.node_weights() {
            node.intent.validate().await?;
        }
        
        Ok(())
    }
    
    // Helper methods
    
    fn update_depths(&mut self, root: NodeIndex) -> Result<()> {
        let mut queue = VecDeque::new();
        queue.push_back((root, 0));
        
        while let Some((current, depth)) = queue.pop_front() {
            self.graph[current].depth = depth;
            
            for edge in self.graph.edges_directed(current, Direction::Outgoing) {
                if *edge.weight() == IntentRelation::Parent {
                    queue.push_back((edge.target(), depth + 1));
                }
            }
        }
        
        Ok(())
    }
}

/// A forest of intent trees for managing multiple independent hierarchies
pub struct IntentForest {
    /// Individual trees in the forest
    trees: Vec<IntentTree>,
    /// Map from intent ID to tree index
    intent_to_tree: HashMap<IntentId, usize>,
}

impl IntentForest {
    /// Creates a new intent forest
    pub fn new() -> Self {
        Self {
            trees: Vec::new(),
            intent_to_tree: HashMap::new(),
        }
    }
    
    /// Adds a new tree to the forest
    pub fn add_tree(&mut self, tree: IntentTree) -> usize {
        let tree_idx = self.trees.len();
        
        // Update intent-to-tree mapping
        for (&intent_id, _) in &tree.node_map {
            self.intent_to_tree.insert(intent_id, tree_idx);
        }
        
        self.trees.push(tree);
        tree_idx
    }
    
    /// Creates a new tree with a root intent
    pub fn create_tree(&mut self, root: HierarchicalIntent) -> usize {
        let mut tree = IntentTree::new();
        let intent_id = root.id();
        tree.add_intent(root);
        
        let tree_idx = self.add_tree(tree);
        self.intent_to_tree.insert(intent_id, tree_idx);
        tree_idx
    }
    
    /// Gets the tree containing an intent
    pub fn get_tree(&self, intent_id: IntentId) -> Option<&IntentTree> {
        self.intent_to_tree
            .get(&intent_id)
            .and_then(|&idx| self.trees.get(idx))
    }
    
    /// Gets a mutable reference to the tree containing an intent
    pub fn get_tree_mut(&mut self, intent_id: IntentId) -> Option<&mut IntentTree> {
        if let Some(&idx) = self.intent_to_tree.get(&intent_id) {
            self.trees.get_mut(idx)
        } else {
            None
        }
    }
    
    /// Merges two trees
    pub fn merge_trees(&mut self, tree1_idx: usize, tree2_idx: usize) -> Result<()> {
        if tree1_idx == tree2_idx {
            return Ok(());
        }
        
        let tree2 = self.trees.get(tree2_idx)
            .ok_or_else(|| IntentError::ValidationFailed("Tree not found".to_string()))?
            .clone();
        
        // Update intent-to-tree mapping  
        for (&intent_id, _) in &tree2.node_map {
            self.intent_to_tree.insert(intent_id, tree1_idx);
        }
        
        // Remove the second tree
        self.trees.remove(tree2_idx);
        
        // Update indices for remaining trees
        for (_intent_id, tree_idx) in &mut self.intent_to_tree {
            if *tree_idx > tree2_idx {
                *tree_idx -= 1;
            }
        }
        
        Ok(())
    }
    
    /// Finds all trees with conflicts
    pub fn find_conflicting_trees(&self) -> Vec<(usize, usize)> {
        let mut conflicts = Vec::new();
        
        for (i, tree1) in self.trees.iter().enumerate() {
            for (j, tree2) in self.trees.iter().enumerate().skip(i + 1) {
                // Check if any intent in tree1 conflicts with any in tree2
                let has_conflict = tree1.graph.node_indices().any(|n1| {
                    let intent1 = tree1.graph[n1].intent.id();
                    tree2.graph.node_indices().any(|n2| {
                        let intent2 = tree2.graph[n2].intent.id();
                        tree1.find_conflicts(intent1).contains(&intent2)
                    })
                });
                
                if has_conflict {
                    conflicts.push((i, j));
                }
            }
        }
        
        conflicts
    }
    
    /// Validates all trees in the forest
    pub async fn validate(&self) -> Result<()> {
        for tree in &self.trees {
            tree.validate().await?;
        }
        Ok(())
    }
    
    /// Gets total number of intents in the forest
    pub fn total_intents(&self) -> usize {
        self.intent_to_tree.len()
    }
    
    /// Gets all root intents across all trees
    pub fn all_roots(&self) -> Vec<IntentId> {
        self.trees
            .iter()
            .flat_map(|tree| tree.get_roots())
            .collect()
    }
}

impl Clone for IntentTree {
    fn clone(&self) -> Self {
        Self {
            graph: self.graph.clone(),
            node_map: self.node_map.clone(),
            roots: self.roots.clone(),
        }
    }
}

impl Default for IntentTree {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for IntentForest {
    fn default() -> Self {
        Self::new()
    }
}