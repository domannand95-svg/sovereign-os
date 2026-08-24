//! ADAM-012 State Mutation & Canonical Storage Subsystem

pub mod journal;
pub mod mutation;
pub mod transition;
pub mod tree;

pub use journal::{JournalStageStatus, StateJournal};
pub use mutation::{compute_delta_digest, normalize_mutations, StateMutation, DELTA_DOMAIN_TAG};
pub use transition::{
    compute_genesis_transition_root, compute_transition_root, StateTransitionReceipt,
    TRANSITION_GENESIS_DOMAIN_TAG, TRANSITION_ROOT_DOMAIN_TAG,
};
pub use tree::{StateTree, STATE_ROOT_DOMAIN};
