#![no_main]

use libfuzzer_sys::fuzz_target;
use sovereign_core_asm::{
    snapshot::{decode as decode_state_snapshot, encode as encode_state_snapshot},
    state::StateVector,
};
use sovereign_ledger::EventRecord;
use sovereign_registry::{IdentityRecord, LineageRecord};

fuzz_target!(|data: &[u8]| {
    // Raw inputs exercise length, discriminator, checksum, and canonical
    // encoding rejection paths. Every decoder must reject malformed input
    // without panicking or reading beyond the supplied slice.
    let _ = decode_state_snapshot(data);
    let _ = EventRecord::decode(data);
    let _ = IdentityRecord::decode(data);
    let _ = LineageRecord::decode(data);

    // State snapshots have a fixed encoded size. Starting from a canonical
    // snapshot and mutating its governed fields lets the fuzzer reach deeper
    // slot-length and padding validation paths instead of stopping only at the
    // outer length check.
    let mut structured_snapshot = encode_state_snapshot(&StateVector::new());
    for (destination, source) in structured_snapshot
        .iter_mut()
        .skip(4)
        .zip(data.iter().copied())
    {
        *destination = source;
    }
    let _ = decode_state_snapshot(&structured_snapshot);
});
