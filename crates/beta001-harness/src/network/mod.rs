//! ADAM-014 Minimal Viable Networking & Inter-Node State Synchronization

pub mod wire;

pub use wire::{
    WireError, WireFrame, WireMessageType, DEFAULT_MAX_WIRE_PAYLOAD_BYTES, WIRE_FORMAT_VERSION_V1,
    WIRE_FRAME_DOMAIN_TAG, WIRE_MAGIC,
};
