//! Single-node Sovereign OS engine bootstrap.
//!
//! Authoritative core state is restored through sovereign-ledger. The registry
//! graph is a deterministic derived projection rebuilt from the same validated
//! ledger. Startup fails closed if the two projections do not reach the same LSN.

use std::error::Error;
use std::fmt;

use sovereign_core_asm::state::{StateTransition, StateVector};
use sovereign_ledger::domain_integration::{
    LedgerEventMapper, LedgerStateTransition, LedgerTransitionError,
};
use sovereign_ledger::{
    initialize_from_snapshot_or_replay, EventRecord, EventType, LedgerAppendEngine, LedgerConfig,
    LedgerError, Lsn, ReplayIterator, RestorationError, RestorationOutcome,
};
use sovereign_policy::{DirectivePolicy, DirectiveRequest, PolicyDecision, PolicyDenial};
use sovereign_registry::{RegistryError, RegistryGraph, RegistryLedgerSync};

/// A successfully booted single-node engine.
pub struct SovereignEngine {
    restoration: Box<RestorationOutcome>,
    registry: RegistryGraph,
    final_lsn: Option<Lsn>,
}

impl SovereignEngine {
    /// Restores authoritative state and rebuilds all derived runtime projections.
    pub fn boot<M>(config: &LedgerConfig, mapper: M) -> Result<Self, EngineBootError<M::Error>>
    where
        M: LedgerEventMapper,
    {
        let restoration = initialize_from_snapshot_or_replay(config, mapper)
            .map_err(EngineBootError::Restoration)?;
        let (registry, projected_final_lsn) =
            rebuild_registry::<M::Error>(config, restoration.final_lsn)?;

        if projected_final_lsn != restoration.final_lsn {
            return Err(EngineBootError::ProjectionTailMismatch {
                restored: restoration.final_lsn,
                projected: projected_final_lsn,
            });
        }

        let final_lsn = restoration.final_lsn;

        Ok(Self {
            restoration: Box::new(restoration),
            registry,
            final_lsn,
        })
    }

    /// Evaluates, preflights, durably appends, and publishes one directive.
    ///
    /// Live state and the registry graph are replaced only after the ledger record
    /// has been appended and synchronized successfully.
    pub fn submit_directive<P, M>(
        &mut self,
        config: &LedgerConfig,
        policy: &P,
        mapper: &M,
        event_type: EventType,
        payload: &[u8],
    ) -> Result<DirectiveReceipt, DirectiveError<M::Error, P::Error>>
    where
        P: DirectivePolicy,
        M: LedgerEventMapper,
    {
        match policy
            .evaluate(DirectiveRequest::new(event_type, payload))
            .map_err(DirectiveError::Policy)?
        {
            PolicyDecision::Allow => {}
            PolicyDecision::Deny(reason) => return Err(DirectiveError::Denied(reason)),
        }

        let mut append =
            LedgerAppendEngine::bootstrap(config.clone()).map_err(DirectiveError::Ledger)?;
        let lsn = append.next_lsn();
        let record = EventRecord {
            lsn,
            event_type,
            payload,
            checksum: 0,
        };

        let transition =
            LedgerStateTransition::from_event(&record, mapper).map_err(DirectiveError::Mapping)?;
        let mut staged_registry = self.registry.clone();
        RegistryLedgerSync::ingest_record(&mut staged_registry, &record)
            .map_err(DirectiveError::Registry)?;

        let mut staged_state = self.restoration.state.clone();
        transition
            .apply(&mut staged_state)
            .map_err(DirectiveError::StateApplication)?;

        let assigned = append
            .append(event_type, payload)
            .map_err(DirectiveError::Ledger)?;
        if assigned != lsn {
            return Err(DirectiveError::Ledger(LedgerError::LsnSequenceGap));
        }

        self.restoration.state = staged_state;
        self.registry = staged_registry;
        self.final_lsn = Some(lsn);

        Ok(DirectiveReceipt { lsn })
    }

    /// Returns the current authoritative state.
    pub fn state(&self) -> &StateVector {
        &self.restoration.state
    }

    /// Returns the deterministic registry projection.
    pub fn registry(&self) -> &RegistryGraph {
        &self.registry
    }

    /// Returns the ledger position shared by all boot projections.
    pub fn final_lsn(&self) -> Option<Lsn> {
        self.final_lsn
    }

    /// Returns restoration source and diagnostic evidence.
    pub fn restoration(&self) -> &RestorationOutcome {
        &self.restoration
    }
}

/// Evidence that one directive was durably admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectiveReceipt {
    pub lsn: Lsn,
}

/// Fail-closed directive-admission errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DirectiveError<MapperError, PolicyError> {
    Policy(PolicyError),
    Denied(PolicyDenial),
    Mapping(MapperError),
    StateApplication(LedgerTransitionError),
    Registry(RegistryError),
    Ledger(LedgerError),
}

impl<MapperError, PolicyError> fmt::Display for DirectiveError<MapperError, PolicyError>
where
    MapperError: fmt::Display,
    PolicyError: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(error) => write!(formatter, "policy evaluation failed: {error}"),
            Self::Denied(reason) => write!(formatter, "directive denied: {reason:?}"),
            Self::Mapping(error) => write!(formatter, "directive mapping failed: {error}"),
            Self::StateApplication(error) => {
                write!(formatter, "directive state application failed: {error:?}")
            }
            Self::Registry(error) => write!(formatter, "registry preflight failed: {error}"),
            Self::Ledger(error) => write!(formatter, "directive persistence failed: {error}"),
        }
    }
}

impl<MapperError, PolicyError> Error for DirectiveError<MapperError, PolicyError>
where
    MapperError: Error + 'static,
    PolicyError: Error + 'static,
{
}

/// Fatal failures that prevent the engine from publishing live state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineBootError<MapperError> {
    /// Authoritative state restoration failed.
    Restoration(RestorationError<MapperError>),
    /// The registry projection could not read the ledger.
    Ledger(LedgerError),
    /// A registry record was invalid or violated graph invariants.
    Registry(RegistryError),
    /// Authoritative and derived projections stopped at different records.
    ProjectionTailMismatch {
        restored: Option<Lsn>,
        projected: Option<Lsn>,
    },
}

impl<MapperError> fmt::Display for EngineBootError<MapperError>
where
    MapperError: fmt::Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Restoration(error) => write!(formatter, "state restoration failed: {error}"),
            Self::Ledger(error) => write!(formatter, "registry ledger replay failed: {error}"),
            Self::Registry(error) => write!(formatter, "registry projection failed: {error}"),
            Self::ProjectionTailMismatch {
                restored,
                projected,
            } => write!(
                formatter,
                "engine projections diverged: restored {restored:?}, projected {projected:?}"
            ),
        }
    }
}

impl<MapperError> Error for EngineBootError<MapperError> where MapperError: Error + 'static {}

fn rebuild_registry<MapperError>(
    config: &LedgerConfig,
    expected_final_lsn: Option<Lsn>,
) -> Result<(RegistryGraph, Option<Lsn>), EngineBootError<MapperError>> {
    let mut graph = RegistryGraph::new();
    if expected_final_lsn.is_none() {
        return Ok((graph, None));
    }

    let mut replay =
        ReplayIterator::bootstrap(config.clone(), Lsn(0)).map_err(EngineBootError::Ledger)?;
    let mut projected_final_lsn = None;

    while let Some(record_result) = replay.next_record() {
        let record = record_result.map_err(EngineBootError::Ledger)?;
        RegistryLedgerSync::ingest_record(&mut graph, &record)
            .map_err(EngineBootError::Registry)?;
        projected_final_lsn = Some(record.lsn);
    }

    Ok((graph, projected_final_lsn))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sovereign_core_asm::state::StateCoordinate;
    use sovereign_ledger::domain_integration::MappedLedgerWrite;
    use sovereign_ledger::{EventRecord, EventType, LedgerAppendEngine};
    use sovereign_policy::EventTypeAllowlist;
    use sovereign_registry::{RegistryNode, RegistryNodeType};
    use std::fs;

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
        let root =
            std::env::temp_dir().join(format!("sovereign_engine_{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        LedgerConfig::new(root)
    }

    #[test]
    fn empty_storage_boots_empty_engine() {
        let config = config("empty");
        let engine = SovereignEngine::boot(&config, LsnMapper).unwrap();

        assert_eq!(engine.final_lsn(), None);
        assert!(engine.registry().is_empty());
        let coordinate = StateCoordinate::new(0).unwrap();
        assert!(engine.state().get(coordinate).is_empty());
    }

    #[test]
    fn boot_restores_state_and_rebuilds_registry_to_same_tail() {
        let config = config("registry_projection");
        let node = RegistryNode::new(
            RegistryNodeType::Capability,
            b"v0.1-capability".to_vec(),
            vec![],
        )
        .unwrap();
        let caid = node.caid();
        let payload = RegistryLedgerSync::serialize_node(&node);

        let mut append = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        assert_eq!(
            append
                .append(EventType::RegistryMutation, &payload)
                .unwrap(),
            Lsn(0)
        );
        append.flush().unwrap();

        let engine = SovereignEngine::boot(&config, LsnMapper).unwrap();

        assert_eq!(engine.final_lsn(), Some(Lsn(0)));
        assert!(engine.registry().contains(&caid));
        let coordinate = StateCoordinate::new(0).unwrap();
        assert_eq!(engine.state().get(coordinate).read_bytes(), payload);

        fs::remove_dir_all(&config.storage_root).unwrap();
    }

    #[test]
    fn malformed_registry_projection_prevents_engine_publication() {
        let config = config("malformed_registry");
        let mut append = LedgerAppendEngine::bootstrap(config.clone()).unwrap();
        append
            .append(EventType::RegistryMutation, b"malformed")
            .unwrap();
        append.flush().unwrap();

        let error = match SovereignEngine::boot(&config, LsnMapper) {
            Err(error) => error,
            Ok(_) => panic!("malformed registry projection unexpectedly booted"),
        };
        assert_eq!(
            error,
            EngineBootError::Registry(RegistryError::SchemaViolation)
        );

        fs::remove_dir_all(&config.storage_root).unwrap();
    }
    #[test]
    fn allowed_directive_persists_publishes_and_reconstructs_after_restart() {
        let config = config("directive_restart");
        let node = RegistryNode::new(
            RegistryNodeType::Capability,
            b"admitted-capability".to_vec(),
            vec![],
        )
        .unwrap();
        let caid = node.caid();
        let payload = RegistryLedgerSync::serialize_node(&node);
        let policy = EventTypeAllowlist::new(&[EventType::RegistryMutation]);
        let mut engine = SovereignEngine::boot(&config, LsnMapper).unwrap();

        let receipt = engine
            .submit_directive(
                &config,
                &policy,
                &LsnMapper,
                EventType::RegistryMutation,
                &payload,
            )
            .unwrap();
        assert_eq!(receipt.lsn, Lsn(0));
        assert_eq!(engine.final_lsn(), Some(Lsn(0)));
        assert!(engine.registry().contains(&caid));
        let live_root = sovereign_ledger::compute_state_root(engine.state());

        drop(engine);
        let restarted = SovereignEngine::boot(&config, LsnMapper).unwrap();
        assert_eq!(restarted.final_lsn(), Some(Lsn(0)));
        assert!(restarted.registry().contains(&caid));
        assert_eq!(
            sovereign_ledger::compute_state_root(restarted.state()),
            live_root
        );
        fs::remove_dir_all(&config.storage_root).unwrap();
    }

    #[test]
    fn denied_directive_changes_neither_memory_nor_ledger() {
        let config = config("directive_denied");
        let node = RegistryNode::new(
            RegistryNodeType::Capability,
            b"denied-capability".to_vec(),
            vec![],
        )
        .unwrap();
        let payload = RegistryLedgerSync::serialize_node(&node);
        let policy = EventTypeAllowlist::denies_all();
        let mut engine = SovereignEngine::boot(&config, LsnMapper).unwrap();
        let before = sovereign_ledger::compute_state_root(engine.state());

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
        assert_eq!(engine.final_lsn(), None);
        assert!(engine.registry().is_empty());
        assert_eq!(sovereign_ledger::compute_state_root(engine.state()), before);
        assert_eq!(
            sovereign_ledger::discover_ledger_tail(&config)
                .unwrap()
                .tail_lsn,
            None
        );
        let _ = fs::remove_dir_all(&config.storage_root);
    }

    #[test]
    fn malformed_registry_directive_fails_preflight_without_persistence() {
        let config = config("directive_malformed");
        let policy = EventTypeAllowlist::new(&[EventType::RegistryMutation]);
        let mut engine = SovereignEngine::boot(&config, LsnMapper).unwrap();
        let before = sovereign_ledger::compute_state_root(engine.state());

        let error = engine
            .submit_directive(
                &config,
                &policy,
                &LsnMapper,
                EventType::RegistryMutation,
                b"malformed",
            )
            .unwrap_err();
        assert_eq!(
            error,
            DirectiveError::Registry(RegistryError::SchemaViolation)
        );
        assert_eq!(engine.final_lsn(), None);
        assert!(engine.registry().is_empty());
        assert_eq!(sovereign_ledger::compute_state_root(engine.state()), before);
        assert_eq!(
            sovereign_ledger::discover_ledger_tail(&config)
                .unwrap()
                .tail_lsn,
            None
        );
        fs::remove_dir_all(&config.storage_root).unwrap();
    }
}
