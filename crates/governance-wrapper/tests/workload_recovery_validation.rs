use governance_wrapper::{GovernanceEngine, Scheduler};
use registry_service::{
    AllocationRequest, CapacityMetrics, OperationalStatus, Registry, WorkloadState,
};
use std::io::Write;
use uuid::Uuid;

fn create_base_node(engine: &mut GovernanceEngine, node_id: Uuid) {
    engine
        .register_node(
            node_id,
            vec!["compute-dense".to_string()],
            CapacityMetrics {
                total_compute_cores: 32,
                allocated_compute_cores: 0,
                total_memory_bytes: 137_438_953_472,
                allocated_memory_bytes: 0,
            },
        )
        .unwrap();

    engine
        .update_status(node_id, OperationalStatus::Active)
        .unwrap();
}

#[test]
fn workload_projection_survives_restart() {
    let ledger_path =
        std::env::temp_dir().join(format!("sovereign_recovery_{}.jsonl", Uuid::new_v4()));

    let node_id = Uuid::new_v4();
    let workload_a = Uuid::new_v4();
    let workload_b = Uuid::new_v4();

    let request_a = AllocationRequest {
        required_compute_cores: 4,
        required_memory_bytes: 17_179_869_184,
        required_capabilities: vec!["compute-dense".to_string()],
    };

    let request_b = AllocationRequest {
        required_compute_cores: 8,
        required_memory_bytes: 34_359_738_368,
        required_capabilities: vec!["compute-dense".to_string()],
    };

    {
        let mut governance = GovernanceEngine::open(&ledger_path).unwrap();
        create_base_node(&mut governance, node_id);

        let mut scheduler = Scheduler::new(&mut governance);

        scheduler
            .schedule_workload(workload_a, 100, request_a)
            .unwrap();
        scheduler
            .schedule_workload(workload_b, 200, request_b)
            .unwrap();
        scheduler.complete_workload(workload_a).unwrap();
    }

    let replayed = Registry::open(&ledger_path).unwrap();

    let a = replayed.get_workload(&workload_a).unwrap();
    assert_eq!(a.state, WorkloadState::Completed);

    let b = replayed.get_workload(&workload_b).unwrap();
    assert_eq!(b.priority, 200);

    match b.state {
        WorkloadState::Running { assigned_node_id } => {
            assert_eq!(assigned_node_id, node_id);
        }
        _ => panic!("workload_b did not recover as running"),
    }

    let node = replayed.get_node(&node_id).unwrap();
    assert_eq!(node.metrics.allocated_compute_cores, 8);
    assert_eq!(node.metrics.allocated_memory_bytes, 34_359_738_368);

    let _ = std::fs::remove_file(ledger_path);
}

#[test]
fn replay_idempotence() {
    let ledger_path =
        std::env::temp_dir().join(format!("sovereign_idempotence_{}.jsonl", Uuid::new_v4()));

    let node_id = Uuid::new_v4();

    {
        let mut governance = GovernanceEngine::open(&ledger_path).unwrap();
        create_base_node(&mut governance, node_id);
    }

    let one = Registry::open(&ledger_path).unwrap();
    let two = Registry::open(&ledger_path).unwrap();

    assert_eq!(one.get_node(&node_id), two.get_node(&node_id));

    let _ = std::fs::remove_file(ledger_path);
}

#[test]
fn interrupted_replay_fails_cleanly() {
    let ledger_path =
        std::env::temp_dir().join(format!("sovereign_corruption_{}.jsonl", Uuid::new_v4()));

    let node_id = Uuid::new_v4();

    {
        let mut governance = GovernanceEngine::open(&ledger_path).unwrap();
        create_base_node(&mut governance, node_id);
    }

    {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&ledger_path)
            .unwrap();

        writeln!(
            file,
            "{{\"action\":\"WORKLOAD_SCHEDULED\",\"payload\":{{ broken"
        )
        .unwrap();
    }

    assert!(Registry::open(&ledger_path).is_err());

    let _ = std::fs::remove_file(ledger_path);
}
