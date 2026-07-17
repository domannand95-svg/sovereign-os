use std::error::Error;
use std::fmt;
use std::fs;

use sovereign_core_asm::state::StateCoordinate;
use sovereign_engine::{DirectiveError, SovereignEngine};
use sovereign_ledger::domain_integration::{LedgerEventMapper, MappedLedgerWrite};
use sovereign_ledger::{compute_state_root, EventRecord, EventType, LedgerConfig, Lsn};
use sovereign_policy::{EventTypeAllowlist, PolicyDenial};
use sovereign_registry::{RegistryLedgerSync, RegistryNode, RegistryNodeType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MappingError {
    InvalidCoordinate,
}

impl fmt::Display for MappingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid state coordinate")
    }
}

impl Error for MappingError {}

#[derive(Clone, Copy)]
struct LsnMapper;

impl LedgerEventMapper for LsnMapper {
    type Error = MappingError;

    fn map<'payload>(
        &self,
        event: &EventRecord<'payload>,
    ) -> Result<MappedLedgerWrite<'payload>, Self::Error> {
        let coordinate = StateCoordinate::new(event.lsn.get() as u32)
            .map_err(|_| MappingError::InvalidCoordinate)?;
        Ok(MappedLedgerWrite::new(coordinate, event.payload))
    }
}

fn config(name: &str) -> LedgerConfig {
    let root = std::env::temp_dir().join(format!(
        "sovereign_boot_restart_{name}_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    LedgerConfig::new(root)
}

#[test]
fn boot_apply_persist_restart_reconstructs_identical_state() {
    let config = config("lifecycle");
    let policy = EventTypeAllowlist::new(&[EventType::RegistryMutation]);
    let root_node =
        RegistryNode::new(RegistryNodeType::Actor, b"v0.1-root-actor".to_vec(), vec![]).unwrap();
    let root_caid = root_node.caid();
    let root_payload = RegistryLedgerSync::serialize_node(&root_node);

    let mut first_boot = SovereignEngine::boot(&config, LsnMapper).unwrap();
    assert_eq!(first_boot.final_lsn(), None);
    let first_receipt = first_boot
        .submit_directive(
            &config,
            &policy,
            &LsnMapper,
            EventType::RegistryMutation,
            &root_payload,
        )
        .unwrap();
    assert_eq!(first_receipt.lsn, Lsn(0));
    assert!(first_boot.registry().contains(&root_caid));
    let root_after_first_persist = compute_state_root(first_boot.state());

    drop(first_boot);

    let mut second_boot = SovereignEngine::boot(&config, LsnMapper).unwrap();
    assert_eq!(second_boot.final_lsn(), Some(Lsn(0)));
    assert!(second_boot.registry().contains(&root_caid));
    assert_eq!(
        compute_state_root(second_boot.state()),
        root_after_first_persist
    );

    let child_node = RegistryNode::new(
        RegistryNodeType::Capability,
        b"v0.1-child-capability".to_vec(),
        vec![root_caid],
    )
    .unwrap();
    let child_caid = child_node.caid();
    let child_payload = RegistryLedgerSync::serialize_node(&child_node);
    let second_receipt = second_boot
        .submit_directive(
            &config,
            &policy,
            &LsnMapper,
            EventType::RegistryMutation,
            &child_payload,
        )
        .unwrap();
    assert_eq!(second_receipt.lsn, Lsn(1));
    assert!(second_boot.registry().contains(&child_caid));
    let root_after_second_persist = compute_state_root(second_boot.state());

    drop(second_boot);

    let final_boot = SovereignEngine::boot(&config, LsnMapper).unwrap();
    assert_eq!(final_boot.final_lsn(), Some(Lsn(1)));
    assert!(final_boot.registry().contains(&root_caid));
    assert!(final_boot.registry().contains(&child_caid));
    assert_eq!(
        compute_state_root(final_boot.state()),
        root_after_second_persist
    );
    assert_eq!(
        final_boot
            .state()
            .get(StateCoordinate::new(0).unwrap())
            .read_bytes(),
        root_payload
    );
    assert_eq!(
        final_boot
            .state()
            .get(StateCoordinate::new(1).unwrap())
            .read_bytes(),
        child_payload
    );

    fs::remove_dir_all(&config.storage_root).unwrap();
}

#[test]
fn denied_directive_remains_absent_after_restart() {
    let config = config("denied");
    let policy = EventTypeAllowlist::denies_all();
    let node = RegistryNode::new(
        RegistryNodeType::Capability,
        b"must-not-persist".to_vec(),
        vec![],
    )
    .unwrap();
    let payload = RegistryLedgerSync::serialize_node(&node);
    let mut engine = SovereignEngine::boot(&config, LsnMapper).unwrap();
    let empty_root = compute_state_root(engine.state());

    let error = engine
        .submit_directive(
            &config,
            &policy,
            &LsnMapper,
            EventType::RegistryMutation,
            &payload,
        )
        .unwrap_err();
    assert_eq!(error, DirectiveError::Denied(PolicyDenial::EventTypeDenied));
    drop(engine);

    let restarted = SovereignEngine::boot(&config, LsnMapper).unwrap();
    assert_eq!(restarted.final_lsn(), None);
    assert!(restarted.registry().is_empty());
    assert_eq!(compute_state_root(restarted.state()), empty_root);
    let _ = fs::remove_dir_all(&config.storage_root);
}

use sovereign_ledger::{AppendCommitStage, LedgerAppendEngine};
use std::process::Command;

const CRASH_ROOT_ENV: &str = "SOVEREIGN_CRASH_TEST_ROOT";
const CRASH_STAGE_ENV: &str = "SOVEREIGN_CRASH_TEST_STAGE";

#[test]
#[ignore = "subprocess crash worker"]
fn crash_worker() {
    let Ok(root) = std::env::var(CRASH_ROOT_ENV) else {
        return;
    };
    let stage_name = std::env::var(CRASH_STAGE_ENV).unwrap();
    let target = match stage_name.as_str() {
        "pending" => AppendCommitStage::PendingSynced,
        "published" => AppendCommitStage::Published,
        "durable" => AppendCommitStage::Durable,
        _ => panic!("unknown crash stage"),
    };

    let config = LedgerConfig::new(root.into());
    let root_node = RegistryNode::new(
        RegistryNodeType::Actor,
        b"crash-root-actor".to_vec(),
        vec![],
    )
    .unwrap();
    let child = RegistryNode::new(
        RegistryNodeType::Capability,
        b"crash-child-capability".to_vec(),
        vec![root_node.caid()],
    )
    .unwrap();
    let payload = RegistryLedgerSync::serialize_node(&child);
    let mut append = LedgerAppendEngine::bootstrap(config).unwrap();
    let _ = append.append_observed(EventType::RegistryMutation, &payload, |stage| {
        if stage == target {
            std::process::abort();
        }
    });
    panic!("crash stage was not reached");
}

#[test]
fn interrupted_publication_exposes_only_previous_or_complete_state() {
    for (stage, should_commit) in [("pending", false), ("published", true), ("durable", true)] {
        let config = config(&format!("crash_{stage}"));
        let policy = EventTypeAllowlist::new(&[EventType::RegistryMutation]);
        let root_node = RegistryNode::new(
            RegistryNodeType::Actor,
            b"crash-root-actor".to_vec(),
            vec![],
        )
        .unwrap();
        let root_caid = root_node.caid();
        let root_payload = RegistryLedgerSync::serialize_node(&root_node);
        let child = RegistryNode::new(
            RegistryNodeType::Capability,
            b"crash-child-capability".to_vec(),
            vec![root_caid],
        )
        .unwrap();
        let child_caid = child.caid();

        let mut engine = SovereignEngine::boot(&config, LsnMapper).unwrap();
        engine
            .submit_directive(
                &config,
                &policy,
                &LsnMapper,
                EventType::RegistryMutation,
                &root_payload,
            )
            .unwrap();
        let previous_root = compute_state_root(engine.state());
        drop(engine);

        let status = Command::new(std::env::current_exe().unwrap())
            .arg("--ignored")
            .arg("--exact")
            .arg("crash_worker")
            .arg("--nocapture")
            .env(CRASH_ROOT_ENV, &config.storage_root)
            .env(CRASH_STAGE_ENV, stage)
            .status()
            .unwrap();
        assert!(!status.success(), "worker did not crash at {stage}");

        let restarted = SovereignEngine::boot(&config, LsnMapper).unwrap();
        assert!(restarted.registry().contains(&root_caid));
        if should_commit {
            assert_eq!(restarted.final_lsn(), Some(Lsn(1)));
            assert!(restarted.registry().contains(&child_caid));
            assert_ne!(compute_state_root(restarted.state()), previous_root);
        } else {
            assert_eq!(restarted.final_lsn(), Some(Lsn(0)));
            assert!(!restarted.registry().contains(&child_caid));
            assert_eq!(compute_state_root(restarted.state()), previous_root);
        }

        fs::remove_dir_all(&config.storage_root).unwrap();
    }
}
