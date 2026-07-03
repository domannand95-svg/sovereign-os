use registry_service::{
    CapacityMetrics, NodeRecord, OperationalStatus, Registry, RegistryEvent,
};
use std::time::Instant;
use uuid::Uuid;

fn node(id: Uuid) -> NodeRecord {
    NodeRecord {
        node_id: id,
        status: OperationalStatus::Active,
        capabilities: vec!["benchmark-node".to_string()],
        metrics: CapacityMetrics {
            total_compute_cores: 4,
            allocated_compute_cores: 0,
            total_memory_bytes: 8_589_934_592,
            allocated_memory_bytes: 0,
        },
    }
}

fn run_case(events: usize) {
    let ledger_path = std::env::temp_dir()
        .join(format!("registry_open_benchmark_{}.jsonl", Uuid::new_v4()));

    let populate_start = Instant::now();

    {
        let mut registry = Registry::open(&ledger_path).unwrap();

        for _ in 0..events {
            registry
                .append_governed_event(RegistryEvent::NodeRegistered {
                    record: node(Uuid::new_v4()),
                })
                .unwrap();
        }
    }

    let populate_elapsed = populate_start.elapsed();

    let open_start = Instant::now();
    let registry = Registry::open(&ledger_path).unwrap();
    let open_elapsed = open_start.elapsed();

    let snapshot_lsn = registry.snapshot_lsn;
    let delta = events.saturating_sub(snapshot_lsn as usize);

    println!(
        "events={} snapshot_lsn={} delta={} populate_ms={:.3} open_ms={:.3}",
        events,
        snapshot_lsn,
        delta,
        populate_elapsed.as_secs_f64() * 1000.0,
        open_elapsed.as_secs_f64() * 1000.0
    );

    assert_eq!(registry.list_nodes().len(), events);

    let _ = std::fs::remove_file(&ledger_path);
    let mut snap_path = ledger_path.clone();
    snap_path.set_extension("snap");
    let _ = std::fs::remove_file(snap_path);
}


#[test]
#[ignore = "manual nominal performance baseline"]
fn open_benchmark_nominal() {
    for events in [500usize, 1_000] {
        run_case(events);
    }
}

#[test]
#[ignore = "resource-intensive scale benchmark"]
fn open_benchmark_extended() {
    for events in [5_000usize, 50_000] {
        run_case(events);
    }
}
