//! Voting mechanisms and quorum certificate handling for HotStuff

use crate::{NodeId, Vote, QuorumCertificate, ViewNumber, VoteType};
use crate::error::{Result, ConsensusError};
use std::collections::{HashMap, HashSet};
use tokio::time::{Duration, Instant};
use tracing::{debug, warn, info};
use uuid::Uuid;

/// Vote collector for aggregating votes into quorum certificates
#[derive(Debug)]
pub struct VoteCollector {
    /// Required quorum size
    quorum_size: usize,
    /// Active vote collections by (view, vote_type, block_id)
    collections: HashMap<(ViewNumber, VoteType, Uuid), VoteCollection>,
    /// Maximum age for vote collections
    max_age: Duration,
}

impl VoteCollector {
    pub fn new(quorum_size: usize) -> Self {
        Self {
            quorum_size,
            collections: HashMap::new(),
            max_age: Duration::from_secs(30), // 30 seconds max age
        }
    }

    /// Add a vote and return QC if quorum is reached
    pub fn add_vote(&mut self, vote: Vote) -> Result<Option<QuorumCertificate>> {
        let key = (vote.view, vote.vote_type.clone(), vote.block_id);
        
        // Get or create vote collection
        let collection = self.collections.entry(key).or_insert_with(|| {
            VoteCollection::new(vote.view, vote.vote_type.clone(), vote.block_id)
        });

        // Add vote to collection
        if collection.add_vote(vote) {
            // Check if we have quorum
            if collection.vote_count() >= self.quorum_size {
                let qc = collection.create_quorum_certificate();
                info!("Formed quorum certificate for view {} with {} votes", 
                      qc.view, qc.vote_count());
                return Ok(Some(qc));
            }
        }

        Ok(None)
    }

    /// Clean up old vote collections
    pub fn cleanup_old_collections(&mut self) {
        let now = Instant::now();
        self.collections.retain(|_, collection| {
            now.duration_since(collection.created_at) < self.max_age
        });
    }

    /// Get vote statistics
    pub fn get_vote_stats(&self) -> VoteStats {
        let total_collections = self.collections.len();
        let total_votes: usize = self.collections.values()
            .map(|c| c.vote_count())
            .sum();
        
        VoteStats {
            total_collections,
            total_votes,
            collections_near_quorum: self.collections.values()
                .filter(|c| c.vote_count() >= (self.quorum_size * 2) / 3)
                .count(),
        }
    }

    /// Clear votes for views older than specified
    pub fn clear_old_views(&mut self, min_view: ViewNumber) {
        self.collections.retain(|(view, _, _), _| *view >= min_view);
    }
}

/// Individual vote collection for a specific (view, vote_type, block_id)
#[derive(Debug)]
struct VoteCollection {
    view: ViewNumber,
    vote_type: VoteType,
    block_id: Uuid,
    votes: HashMap<NodeId, Vote>,
    created_at: Instant,
}

impl VoteCollection {
    fn new(view: ViewNumber, vote_type: VoteType, block_id: Uuid) -> Self {
        Self {
            view,
            vote_type,
            block_id,
            votes: HashMap::new(),
            created_at: Instant::now(),
        }
    }

    /// Add a vote, returns true if new, false if duplicate
    fn add_vote(&mut self, vote: Vote) -> bool {
        // Validate vote matches collection
        if vote.view != self.view || 
           vote.vote_type != self.vote_type || 
           vote.block_id != self.block_id {
            warn!("Vote mismatch: expected view={}, type={:?}, block={}, got view={}, type={:?}, block={}",
                  self.view, self.vote_type, self.block_id,
                  vote.view, vote.vote_type, vote.block_id);
            return false;
        }

        // Check if this is a new vote
        if self.votes.contains_key(&vote.voter) {
            debug!("Duplicate vote from {} for view {}", vote.voter, vote.view);
            return false;
        }

        self.votes.insert(vote.voter.clone(), vote);
        true
    }

    fn vote_count(&self) -> usize {
        self.votes.len()
    }

    fn create_quorum_certificate(&self) -> QuorumCertificate {
        QuorumCertificate::new(self.votes.values().cloned().collect())
    }
}

/// Vote statistics for monitoring
#[derive(Debug, Clone)]
pub struct VoteStats {
    pub total_collections: usize,
    pub total_votes: usize,
    pub collections_near_quorum: usize,
}

/// Validator for verifying vote integrity and safety
#[derive(Debug)]
pub struct VoteValidator {
    /// Known validators
    validators: HashSet<NodeId>,
    /// Byzantine fault threshold
    byzantine_threshold: usize,
}

impl VoteValidator {
    pub fn new(validators: Vec<NodeId>, byzantine_threshold: usize) -> Self {
        Self {
            validators: validators.into_iter().collect(),
            byzantine_threshold,
        }
    }

    /// Validate a vote for basic correctness
    pub fn validate_vote(&self, vote: &Vote) -> Result<()> {
        // Check if voter is a known validator
        if !self.validators.contains(&vote.voter) {
            return Err(ConsensusError::UnknownValidator(vote.voter.clone()));
        }

        // Check vote timestamp is reasonable (within 5 minutes)
        let now = chrono::Utc::now();
        let max_age = chrono::Duration::minutes(5);
        if now.signed_duration_since(vote.timestamp) > max_age {
            return Err(ConsensusError::InvalidTimestamp);
        }

        // Additional validations can be added here
        Ok(())
    }

    /// Validate a quorum certificate
    pub fn validate_qc(&self, qc: &QuorumCertificate) -> Result<()> {
        // Check minimum vote count
        let required_votes = 2 * self.byzantine_threshold + 1;
        if qc.votes.len() < required_votes {
            return Err(ConsensusError::InsufficientVotes { 
                required: required_votes, 
                received: qc.votes.len() 
            });
        }

        // Check all votes are for the same block and view
        let first_vote = qc.votes.first()
            .ok_or(ConsensusError::EmptyQuorumCertificate)?;
        
        for vote in &qc.votes {
            if vote.view != first_vote.view ||
               vote.block_id != first_vote.block_id ||
               vote.vote_type != first_vote.vote_type {
                return Err(ConsensusError::InconsistentVotes);
            }

            // Validate each individual vote
            self.validate_vote(vote)?;
        }

        // Check for duplicate voters
        let mut voters = HashSet::new();
        for vote in &qc.votes {
            if !voters.insert(vote.voter.clone()) {
                return Err(ConsensusError::DuplicateVoter(vote.voter.clone()));
            }
        }

        Ok(())
    }

    /// Update the validator set
    pub fn update_validators(&mut self, validators: Vec<NodeId>) {
        self.validators = validators.into_iter().collect();
    }
}

/// Vote aggregation strategy
pub trait VoteAggregationStrategy {
    /// Determine if votes should be aggregated into a QC
    fn should_aggregate(&self, votes: &[Vote], required_count: usize) -> bool;
    
    /// Create aggregated signature if supported
    fn aggregate_signatures(&self, votes: &[Vote]) -> Option<Vec<u8>>;
}

/// Simple threshold-based aggregation
#[derive(Debug, Clone)]
pub struct ThresholdAggregation;

impl VoteAggregationStrategy for ThresholdAggregation {
    fn should_aggregate(&self, votes: &[Vote], required_count: usize) -> bool {
        votes.len() >= required_count
    }

    fn aggregate_signatures(&self, _votes: &[Vote]) -> Option<Vec<u8>> {
        // Simple aggregation - just concatenate (real implementation would use BLS)
        None // Disabled for now
    }
}

/// BLS signature aggregation (placeholder for future implementation)
#[derive(Debug, Clone)]
pub struct BlsAggregation {
    _enabled: bool,
}

impl BlsAggregation {
    pub fn new() -> Self {
        Self { _enabled: false }
    }
}

impl VoteAggregationStrategy for BlsAggregation {
    fn should_aggregate(&self, votes: &[Vote], required_count: usize) -> bool {
        votes.len() >= required_count
    }

    fn aggregate_signatures(&self, votes: &[Vote]) -> Option<Vec<u8>> {
        // TODO: Implement BLS signature aggregation
        // For now, return None to use individual signatures
        if votes.is_empty() {
            None
        } else {
            // Placeholder aggregation
            let mut aggregated = Vec::new();
            for vote in votes {
                aggregated.extend_from_slice(&vote.signature);
            }
            Some(aggregated)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Transaction;
    use chrono::Utc;

    fn create_test_vote(voter: NodeId, view: ViewNumber, vote_type: VoteType, block_id: Uuid) -> Vote {
        Vote {
            vote_type,
            view,
            block_id,
            voter,
            signature: vec![1, 2, 3, 4], // Mock signature
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_vote_collector() {
        let mut collector = VoteCollector::new(3); // Quorum of 3
        let block_id = Uuid::new_v4();
        let view = ViewNumber::new(1);
        
        // Add votes one by one
        let vote1 = create_test_vote(NodeId::new(), view, VoteType::Prepare, block_id);
        let vote2 = create_test_vote(NodeId::new(), view, VoteType::Prepare, block_id);
        let vote3 = create_test_vote(NodeId::new(), view, VoteType::Prepare, block_id);
        
        // First two votes shouldn't form QC
        assert!(collector.add_vote(vote1).unwrap().is_none());
        assert!(collector.add_vote(vote2).unwrap().is_none());
        
        // Third vote should form QC
        let qc = collector.add_vote(vote3).unwrap();
        assert!(qc.is_some());
        assert_eq!(qc.unwrap().vote_count(), 3);
    }

    #[test]
    fn test_vote_validator() {
        let validators = vec![NodeId::new(), NodeId::new(), NodeId::new()];
        let validator = VoteValidator::new(validators.clone(), 1);
        
        let block_id = Uuid::new_v4();
        let view = ViewNumber::new(1);
        
        // Valid vote
        let valid_vote = create_test_vote(validators[0].clone(), view, VoteType::Prepare, block_id);
        assert!(validator.validate_vote(&valid_vote).is_ok());
        
        // Invalid voter
        let invalid_vote = create_test_vote(NodeId::new(), view, VoteType::Prepare, block_id);
        assert!(validator.validate_vote(&invalid_vote).is_err());
    }

    #[test]
    fn test_duplicate_vote_handling() {
        let mut collector = VoteCollector::new(2);
        let block_id = Uuid::new_v4();
        let view = ViewNumber::new(1);
        let voter = NodeId::new();
        
        let vote1 = create_test_vote(voter.clone(), view, VoteType::Prepare, block_id);
        let vote2 = create_test_vote(voter.clone(), view, VoteType::Prepare, block_id);
        
        // First vote should be accepted
        assert!(collector.add_vote(vote1).unwrap().is_none());
        
        // Second vote from same voter should be ignored
        assert!(collector.add_vote(vote2).unwrap().is_none());
        
        // Stats should show only 1 vote
        let stats = collector.get_vote_stats();
        assert_eq!(stats.total_votes, 1);
    }

    #[test]
    fn test_qc_validation() {
        let validators = vec![NodeId::new(), NodeId::new(), NodeId::new()];
        let validator = VoteValidator::new(validators.clone(), 1);
        
        let block_id = Uuid::new_v4();
        let view = ViewNumber::new(1);
        
        // Create valid QC
        let votes = vec![
            create_test_vote(validators[0].clone(), view, VoteType::Prepare, block_id),
            create_test_vote(validators[1].clone(), view, VoteType::Prepare, block_id),
            create_test_vote(validators[2].clone(), view, VoteType::Prepare, block_id),
        ];
        
        let qc = QuorumCertificate::new(votes);
        assert!(validator.validate_qc(&qc).is_ok());
    }

    #[test]
    fn test_threshold_aggregation() {
        let strategy = ThresholdAggregation;
        let votes = vec![
            create_test_vote(NodeId::new(), ViewNumber::new(1), VoteType::Prepare, Uuid::new_v4()),
            create_test_vote(NodeId::new(), ViewNumber::new(1), VoteType::Prepare, Uuid::new_v4()),
            create_test_vote(NodeId::new(), ViewNumber::new(1), VoteType::Prepare, Uuid::new_v4()),
        ];
        
        assert!(strategy.should_aggregate(&votes, 3));
        assert!(!strategy.should_aggregate(&votes, 4));
    }
}
