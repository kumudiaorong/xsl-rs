pub mod rbtree;
pub use rbtree::RBTreeMap;

pub mod rbtree_map {
    pub use super::rbtree::{Entry, OccupiedEntry, VacantEntry};
}
