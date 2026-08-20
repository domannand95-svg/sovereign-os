// =====================================================================
// 1. PUBLICATION RISK CLASSIFICATION DOMAIN TYPES & CONTRACTS
// =====================================================================

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublicationRiskClassification {
    StandardSourceChange = 0,
    SensitiveConfigurationChange = 1,
    ReleaseAutomationChange = 2,
    DeploymentInfrastructureChange = 3,
    CiWorkflowChange = 4,
    UnknownSensitiveChange = 5,
}

pub struct PublicationRiskClassifier;

impl PublicationRiskClassifier {
    pub fn classify_path(raw_path: &str) -> PublicationRiskClassification {
        // Normalize path: convert backslashes and remove leading/trailing whitespace or traversal attempts
        let cleaned = raw_path.replace('\\', "/");

        // Defend against path traversal manipulation (TC-C003-005)
        if cleaned.contains("..") || cleaned.starts_with('/') {
            return PublicationRiskClassification::UnknownSensitiveChange;
        }

        let lower = cleaned.to_lowercase();

        // CI Workflow Authority (.github/workflows/**, .github/actions/**)
        if lower.starts_with(".github/workflows/") || lower.starts_with(".github/actions/") {
            return PublicationRiskClassification::CiWorkflowChange;
        }

        // Deployment Infrastructure (terraform/**, infra/**, deployment/**, k8s/**, helm/**, Dockerfile, docker-compose*.yml)
        if lower.starts_with("terraform/")
            || lower.starts_with("infra/")
            || lower.starts_with("deployment/")
            || lower.starts_with("k8s/")
            || lower.starts_with("helm/")
            || lower == "dockerfile"
            || lower.starts_with("dockerfile/")
            || lower.starts_with("docker-compose")
            || lower.contains("/docker-compose")
        {
            return PublicationRiskClassification::DeploymentInfrastructureChange;
        }

        // Release Automation (.github/release/**, release/**, scripts/release/**)
        if lower.starts_with(".github/release/")
            || lower.starts_with("release/")
            || lower.starts_with("scripts/release/")
        {
            return PublicationRiskClassification::ReleaseAutomationChange;
        }

        // Dependency Supply Chain (Cargo.toml, Cargo.lock, package.json, package-lock.json, requirements*.txt, go.mod, go.sum)
        if lower == "cargo.toml"
            || lower == "cargo.lock"
            || lower == "package.json"
            || lower == "package-lock.json"
            || lower == "go.mod"
            || lower == "go.sum"
            || lower.starts_with("requirements")
            || lower.contains("/requirements")
        {
            return PublicationRiskClassification::SensitiveConfigurationChange;
        }

        PublicationRiskClassification::StandardSourceChange
    }

    pub fn classify_candidate(changed_paths: &[String]) -> PublicationRiskClassification {
        if changed_paths.is_empty() {
            return PublicationRiskClassification::StandardSourceChange;
        }

        changed_paths
            .iter()
            .map(|p| Self::classify_path(p))
            .max()
            .unwrap_or(PublicationRiskClassification::StandardSourceChange)
    }
}

// =====================================================================
// 2. C-003 PUBLICATION RISK CLASSIFICATION TEST SUITES
// =====================================================================

#[cfg(test)]
mod c003_risk_tests {
    use super::*;

    #[test]
    fn tc_c003_001_normal_source_change() {
        let paths = vec!["src/lib.rs".to_string(), "README.md".to_string()];
        let classification = PublicationRiskClassifier::classify_candidate(&paths);
        assert_eq!(
            classification,
            PublicationRiskClassification::StandardSourceChange
        );
    }

    #[test]
    fn tc_c003_002_ci_workflow_modification() {
        let paths = vec![
            "src/main.rs".to_string(),
            ".github/workflows/release.yml".to_string(),
        ];
        let classification = PublicationRiskClassifier::classify_candidate(&paths);
        assert_eq!(
            classification,
            PublicationRiskClassification::CiWorkflowChange
        );
    }

    #[test]
    fn tc_c003_003_deployment_manifest_modification() {
        let paths = vec!["k8s/production.yaml".to_string()];
        let classification = PublicationRiskClassifier::classify_candidate(&paths);
        assert_eq!(
            classification,
            PublicationRiskClassification::DeploymentInfrastructureChange
        );
    }

    #[test]
    fn tc_c003_004_mixed_change_set_surfaces_highest_risk() {
        let paths = vec![
            "src/main.rs".to_string(),
            "Cargo.toml".to_string(),
            ".github/workflows/test.yml".to_string(),
        ];
        let classification = PublicationRiskClassifier::classify_candidate(&paths);
        assert_eq!(
            classification,
            PublicationRiskClassification::CiWorkflowChange
        );
    }

    #[test]
    fn tc_c003_005_path_traversal_attempt_yields_unknown_sensitive() {
        let paths = vec!["../../.github/workflows/deploy.yml".to_string()];
        let classification = PublicationRiskClassifier::classify_candidate(&paths);
        assert_eq!(
            classification,
            PublicationRiskClassification::UnknownSensitiveChange
        );
    }

    #[test]
    fn tc_c003_006_case_and_encoding_manipulation_handled_deterministically() {
        // Case manipulation
        let path_upper = ".GITHUB/workflows/test.yml".to_string();
        assert_eq!(
            PublicationRiskClassifier::classify_path(&path_upper),
            PublicationRiskClassification::CiWorkflowChange
        );

        // Windows path separator normalization
        let path_win = "k8s\\production.yaml".to_string();
        assert_eq!(
            PublicationRiskClassifier::classify_path(&path_win),
            PublicationRiskClassification::DeploymentInfrastructureChange
        );
    }
}
