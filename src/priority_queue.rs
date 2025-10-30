//! Priority queue for tracking user health (min-heap by health)

use priority_queue::PriorityQueue;
use solana_sdk::pubkey::Pubkey;
use std::cmp::Reverse;
use std::collections::HashMap;

/// User health snapshot
#[derive(Debug, Clone)]
pub struct UserHealth {
    /// User pubkey
    pub user: Pubkey,
    /// Portfolio pubkey
    pub portfolio: Pubkey,
    /// Health = equity - MM
    pub health: i128,
    /// Equity (including unrealized PnL)
    pub equity: i128,
    /// Maintenance margin requirement
    pub mm: u128,
    /// Last update timestamp
    pub last_update: u64,
}

impl UserHealth {
    /// Check if user needs liquidation
    pub fn needs_liquidation(&self, threshold: i128) -> bool {
        self.health <= threshold
    }

    /// Check if user is in pre-liquidation zone
    pub fn in_preliq_zone(&self, buffer: i128) -> bool {
        self.health > 0 && self.health < buffer
    }
}

/// Health-based priority queue (min-heap: lowest health first)
pub struct HealthQueue {
    /// Priority queue (using Reverse for min-heap)
    queue: PriorityQueue<Pubkey, Reverse<i128>>,
    /// Map for O(1) lookups
    map: HashMap<Pubkey, UserHealth>,
}

impl HealthQueue {
    /// Create new empty queue
    pub fn new() -> Self {
        Self {
            queue: PriorityQueue::new(),
            map: HashMap::new(),
        }
    }

    /// Push or update user health
    pub fn push(&mut self, user_health: UserHealth) {
        let user = user_health.user;
        let health = user_health.health;

        // Update map
        self.map.insert(user, user_health);

        // Update priority queue (using Reverse for min-heap)
        self.queue.push(user, Reverse(health));
    }

    /// Pop user with lowest health
    pub fn pop(&mut self) -> Option<UserHealth> {
        let (user, _priority) = self.queue.pop()?;
        self.map.remove(&user)
    }

    /// Peek at user with lowest health without removing
    pub fn peek(&self) -> Option<&UserHealth> {
        let (user, _priority) = self.queue.peek()?;
        self.map.get(user)
    }

    /// Update existing user health
    pub fn update(&mut self, user: &Pubkey, new_health: UserHealth) {
        self.push(new_health);
    }

    /// Remove user from queue
    pub fn remove(&mut self, user: &Pubkey) -> Option<UserHealth> {
        self.queue.remove(user);
        self.map.remove(user)
    }

    /// Get user health by pubkey
    pub fn get(&self, user: &Pubkey) -> Option<&UserHealth> {
        self.map.get(user)
    }

    /// Check if queue contains user
    pub fn contains(&self, user: &Pubkey) -> bool {
        self.map.contains_key(user)
    }

    /// Get number of users in queue
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Get all users below health threshold (need liquidation)
    pub fn get_liquidatable(&self, threshold: i128) -> Vec<UserHealth> {
        self.map
            .values()
            .filter(|uh| uh.needs_liquidation(threshold))
            .cloned()
            .collect()
    }

    /// Get users in pre-liquidation zone
    pub fn get_preliq_candidates(&self, buffer: i128) -> Vec<UserHealth> {
        self.map
            .values()
            .filter(|uh| uh.in_preliq_zone(buffer))
            .cloned()
            .collect()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.queue.clear();
        self.map.clear();
    }
}

impl Default for HealthQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::pubkey::Pubkey;

    fn make_user_health(user_idx: u8, health: i128) -> UserHealth {
        UserHealth {
            user: Pubkey::new_unique(),
            portfolio: Pubkey::new_unique(),
            health,
            equity: health + 100_000_000,
            mm: 100_000_000,
            last_update: 0,
        }
    }

    #[test]
    fn test_queue_push_pop() {
        let mut queue = HealthQueue::new();

        let uh1 = make_user_health(1, -5_000_000);
        let uh2 = make_user_health(2, 10_000_000);
        let uh3 = make_user_health(3, -10_000_000);

        queue.push(uh1.clone());
        queue.push(uh2.clone());
        queue.push(uh3.clone());

        assert_eq!(queue.len(), 3);

        // Should pop lowest health first (-10M)
        let popped = queue.pop().unwrap();
        assert_eq!(popped.health, -10_000_000);

        // Next should be -5M
        let popped = queue.pop().unwrap();
        assert_eq!(popped.health, -5_000_000);
    }

    #[test]
    fn test_queue_peek() {
        let mut queue = HealthQueue::new();

        let uh1 = make_user_health(1, 5_000_000);
        let uh2 = make_user_health(2, -5_000_000);

        queue.push(uh1);
        queue.push(uh2);

        // Peek should return lowest without removing
        let peeked = queue.peek().unwrap();
        assert_eq!(peeked.health, -5_000_000);

        // Queue should still have 2 elements
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_liquidatable_users() {
        let mut queue = HealthQueue::new();

        queue.push(make_user_health(1, -5_000_000));  // Needs liq
        queue.push(make_user_health(2, 5_000_000));   // Healthy
        queue.push(make_user_health(3, -1_000_000));  // Needs liq

        let liquidatable = queue.get_liquidatable(0);
        assert_eq!(liquidatable.len(), 2);
    }

    #[test]
    fn test_preliq_candidates() {
        let mut queue = HealthQueue::new();

        let buffer = 10_000_000; // $10 buffer

        queue.push(make_user_health(1, 5_000_000));   // In preliq zone
        queue.push(make_user_health(2, 15_000_000));  // Healthy
        queue.push(make_user_health(3, -5_000_000));  // Below MM

        let preliq = queue.get_preliq_candidates(buffer);
        assert_eq!(preliq.len(), 1);
        assert_eq!(preliq[0].health, 5_000_000);
    }

    #[test]
    fn test_queue_update() {
        let mut queue = HealthQueue::new();

        let user = Pubkey::new_unique();
        let mut uh = UserHealth {
            user,
            portfolio: Pubkey::new_unique(),
            health: 10_000_000,
            equity: 110_000_000,
            mm: 100_000_000,
            last_update: 0,
        };

        queue.push(uh.clone());

        // Update health
        uh.health = -5_000_000;
        uh.equity = 95_000_000;
        queue.update(&user, uh);

        let retrieved = queue.get(&user).unwrap();
        assert_eq!(retrieved.health, -5_000_000);
    }
}
