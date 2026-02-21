//! Acceptance tests for stage-scoped write allowlists (src-14x).
//!
//! These tests encode the public contract invariants:
//! 1. Path validation is performed BEFORE persistence
//! 2. Allowlist is role-based and deterministic
//! 3. Contract stage accepts ONLY .beads/contracts/<bead_id>.cue
//! 4. AcceptanceTest stage accepts ONLY approved test locations
//! 5. Implementation stage CANNOT write workflow orchestration files
//! 6. Paths escaping workspace root are BLOCKED

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use proptest::prelude::*;
use std::path::{Path, PathBuf};

pub use oya::runtime_tools::write_allowlist::{
    is_write_allowed, validate_write_path, StageWriteConfig, WriteAllowlistError,
};
pub use oya::types::StageName;

const WORKSPACE_ROOT: &str = "/home/user/project";

fn workspace() -> PathBuf {
    PathBuf::from(WORKSPACE_ROOT)
}

fn bead_id_strategy() -> impl Strategy<Value = String> {
    "[a-z0-9]{8,20}"
}

fn path_segment_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_-]{1,20}"
}

fn arbitrary_path_strategy() -> impl Strategy<Value = PathBuf> {
    prop::collection::vec(path_segment_strategy(), 1..5).prop_map(|segments| {
        let mut path = PathBuf::from(WORKSPACE_ROOT);
        for seg in segments {
            path.push(seg);
        }
        path
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn prop_contract_stage_rejects_non_beads_contracts_paths(
        path_segment in path_segment_strategy(),
        file_name in path_segment_strategy(),
    ) {
        let ws = workspace();

        let disallowed_paths = vec![
            ws.join("docs").join(&path_segment).join(format!("{}.md", file_name)),
            ws.join("docs").join(format!("{}.md", file_name)),
            ws.join(&format!("{}.md", file_name)),
            ws.join("README.md"),
            ws.join("src").join(&file_name),
            ws.join("tests").join(&file_name),
        ];

        for path in disallowed_paths {
            let result = validate_write_path(&StageName::Contract, &path, &ws);
            prop_assert!(
                matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
                "Contract stage incorrectly allowed: {:?}",
                path
            );
        }
    }

    #[test]
    fn prop_contract_stage_accepts_only_beads_contracts_cue(
        bead_id in bead_id_strategy(),
    ) {
        let ws = workspace();
        let allowed_path = ws.join(".beads").join("contracts").join(format!("{}.cue", bead_id));

        let result = validate_write_path(&StageName::Contract, &allowed_path, &ws);
        prop_assert!(
            result.is_ok(),
            "Contract stage should allow .beads/contracts/<bead_id>.cue, rejected: {:?}",
            result
        );
    }

    #[test]
    fn prop_contract_stage_rejects_beads_contracts_without_cue_extension(
        bead_id in bead_id_strategy(),
        ext in "[a-z]{2,4}",
    ) {
        if ext == "cue" {
            return Ok(());
        }

        let ws = workspace();
        let path = ws.join(".beads").join("contracts").join(format!("{}.{}", bead_id, ext));

        let result = validate_write_path(&StageName::Contract, &path, &ws);
        prop_assert!(
            matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
            "Contract stage should reject non-.cue files in .beads/contracts/: {:?}",
            path
        );
    }

    #[test]
    fn prop_acceptance_test_stage_accepts_only_approved_locations(
        test_name in path_segment_strategy(),
    ) {
        let ws = workspace();

        let approved_paths = vec![
            ws.join("tests").join(format!("{}_test.rs", test_name)),
            ws.join("tests").join(format!("test_{}.rs", test_name)),
            ws.join("tests").join("integration").join(format!("{}.rs", test_name)),
            ws.join("tests").join("unit").join(format!("{}.rs", test_name)),
        ];

        for path in approved_paths {
            let result = validate_write_path(&StageName::AcceptanceTest, &path, &ws);
            prop_assert!(
                result.is_ok(),
                "AcceptanceTest stage should allow approved test location: {:?}",
                path
            );
        }
    }

    #[test]
    fn prop_acceptance_test_stage_rejects_src_directory(
        file_name in path_segment_strategy(),
    ) {
        let ws = workspace();
        let path = ws.join("src").join(format!("{}.rs", file_name));

        let result = validate_write_path(&StageName::AcceptanceTest, &path, &ws);
        prop_assert!(
            matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
            "AcceptanceTest stage should reject src/ writes: {:?}",
            path
        );
    }

    #[test]
    fn prop_implementation_stage_rejects_workflow_orchestration_files(
        file_name in path_segment_strategy(),
    ) {
        let ws = workspace();

        let workflow_paths = vec![
            ws.join(".github").join("workflows").join(format!("{}.yml", file_name)),
            ws.join(".github").join("actions").join(&file_name),
            ws.join("scripts").join(format!("{}.sh", file_name)),
            ws.join(".moon").join("tasks.yml"),
            ws.join("Makefile"),
            ws.join("Taskfile.yml"),
            ws.join(".gitlab-ci.yml"),
            ws.join("Jenkinsfile"),
        ];

        for path in workflow_paths {
            let result = validate_write_path(&StageName::Implementation, &path, &ws);
            prop_assert!(
                matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
                "Implementation stage should reject workflow orchestration files: {:?}",
                path
            );
        }
    }

    #[test]
    fn prop_implementation_stage_accepts_src_and_benches(
        file_name in path_segment_strategy(),
    ) {
        let ws = workspace();

        let allowed_paths = vec![
            ws.join("src").join(format!("{}.rs", file_name)),
            ws.join("src").join(&file_name).join("mod.rs"),
            ws.join("benches").join(format!("{}.rs", file_name)),
            ws.join("src").join("lib.rs"),
            ws.join("src").join("main.rs"),
        ];

        for path in allowed_paths {
            let result = validate_write_path(&StageName::Implementation, &path, &ws);
            prop_assert!(
                result.is_ok(),
                "Implementation stage should allow src/ and benches/: {:?}",
                path
            );
        }
    }

    #[test]
    fn prop_path_traversal_is_blocked(
        traversal_depth in 1usize..10,
        file_name in path_segment_strategy(),
    ) {
        let ws = workspace();

        let traversal = "../".repeat(traversal_depth);
        let path_str = format!("{}/{}{}", WORKSPACE_ROOT, traversal, file_name);
        let path = PathBuf::from(&path_str);

        let result = validate_write_path(&StageName::Implementation, &path, &ws);
        prop_assert!(
            matches!(result, Err(WriteAllowlistError::PathTraversalDetected(_))),
            "Path traversal should be blocked: {:?}",
            path_str
        );
    }

    #[test]
    fn prop_escape_workspace_root_is_blocked(
        sibling_name in path_segment_strategy(),
        file_name in path_segment_strategy(),
    ) {
        let ws = workspace();
        let sibling_path = PathBuf::from(format!("/home/user/{}-other/{}", sibling_name, file_name));

        let result = validate_write_path(&StageName::Implementation, &sibling_path, &ws);
        prop_assert!(
            matches!(result, Err(WriteAllowlistError::NotWithinWorkspace { .. })),
            "Escape from workspace should be blocked: {:?}",
            sibling_path
        );
    }

    #[test]
    fn prop_review_stage_is_always_read_only(path in arbitrary_path_strategy()) {
        let ws = workspace();

        let result = validate_write_path(&StageName::Review, &path, &ws);
        prop_assert!(
            matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
            "Review stage should always be read-only: {:?}",
            path
        );
    }

    #[test]
    fn prop_stage_write_config_is_deterministic(stage in prop_oneof![
        Just(StageName::Contract),
        Just(StageName::AcceptanceTest),
        Just(StageName::Implementation),
        Just(StageName::Review),
        Just(StageName::ShipGate),
    ]) {
        let config1 = StageWriteConfig::for_stage(stage.clone());
        let config2 = StageWriteConfig::for_stage(stage);

        prop_assert_eq!(config1, config2, "StageWriteConfig must be deterministic");
        prop_assert_eq!(config1.stage, config2.stage);
    }

    #[test]
    fn prop_validation_is_performed_before_stage_check(
        path_segment in path_segment_strategy(),
    ) {
        let ws = workspace();

        let path_with_traversal = PathBuf::from(format!("{}/../etc/passwd", WORKSPACE_ROOT));

        let result = validate_write_path(&StageName::Implementation, &path_with_traversal, &ws);

        prop_assert!(
            !matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
            "Path validation must happen BEFORE stage check - traversal should be caught first"
        );
        prop_assert!(
            matches!(result, Err(WriteAllowlistError::PathTraversalDetected(_))),
            "Traversal detection must precede stage write check"
        );
    }
}

#[test]
fn contract_stage_accepts_only_beads_contracts_cue_specific() {
    let ws = workspace();

    let allowed_path = ws.join(".beads").join("contracts").join("oya-20260220154435-ja0wkiie.cue");
    let result = validate_write_path(&StageName::Contract, &allowed_path, &ws);
    assert!(result.is_ok(), "Contract should allow .beads/contracts/<bead_id>.cue");
}

#[test]
fn contract_stage_rejects_docs_directory() {
    let ws = workspace();
    let docs_path = ws.join("docs").join("contract.md");

    let result = validate_write_path(&StageName::Contract, &docs_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
        "Contract stage must NOT allow docs/ - only .beads/contracts/"
    );
}

#[test]
fn contract_stage_rejects_root_markdown_files() {
    let ws = workspace();
    let readme_path = ws.join("README.md");

    let result = validate_write_path(&StageName::Contract, &readme_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
        "Contract stage must NOT allow root *.md files"
    );
}

#[test]
fn contract_stage_rejects_beads_root() {
    let ws = workspace();
    let beads_path = ws.join(".beads").join("config.json");

    let result = validate_write_path(&StageName::Contract, &beads_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
        "Contract stage must only allow .beads/contracts/, not .beads/ root"
    );
}

#[test]
fn contract_stage_rejects_beads_contracts_json() {
    let ws = workspace();
    let json_path = ws.join(".beads").join("contracts").join("bead-123.json");

    let result = validate_write_path(&StageName::Contract, &json_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
        "Contract stage must only allow .cue extension, not .json"
    );
}

#[test]
fn acceptance_test_stage_accepts_tests_subdirectories() {
    let ws = workspace();

    let approved_paths = vec![
        ws.join("tests").join("integration_test.rs"),
        ws.join("tests").join("unit").join("module_test.rs"),
        ws.join("tests").join("e2e").join("scenario_test.rs"),
        ws.join("tests").join("acceptance").join("feature_test.rs"),
    ];

    for path in approved_paths {
        let result = validate_write_path(&StageName::AcceptanceTest, &path, &ws);
        assert!(result.is_ok(), "AcceptanceTest should allow tests/ subdirectories: {:?}", path);
    }
}

#[test]
fn acceptance_test_stage_rejects_root_test_patterns() {
    let ws = workspace();

    let disallowed_paths = vec![ws.join("module_test.rs"), ws.join("tests.rs"), ws.join("mod.rs")];

    for path in disallowed_paths {
        let result = validate_write_path(&StageName::AcceptanceTest, &path, &ws);
        assert!(
            matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
            "AcceptanceTest must only allow tests/ directory, not root files: {:?}",
            path
        );
    }
}

#[test]
fn implementation_stage_rejects_github_workflows() {
    let ws = workspace();
    let workflow_path = ws.join(".github").join("workflows").join("ci.yml");

    let result = validate_write_path(&StageName::Implementation, &workflow_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
        "Implementation must NOT write workflow orchestration files (.github/workflows/)"
    );
}

#[test]
fn implementation_stage_rejects_moon_configs() {
    let ws = workspace();
    let moon_path = ws.join(".moon").join("tasks.yml");

    let result = validate_write_path(&StageName::Implementation, &moon_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
        "Implementation must NOT write build system configs (.moon/)"
    );
}

#[test]
fn implementation_stage_rejects_scripts_directory() {
    let ws = workspace();
    let script_path = ws.join("scripts").join("deploy.sh");

    let result = validate_write_path(&StageName::Implementation, &script_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
        "Implementation must NOT write to scripts/ (orchestration files)"
    );
}

#[test]
fn implementation_stage_rejects_makefile() {
    let ws = workspace();
    let makefile_path = ws.join("Makefile");

    let result = validate_write_path(&StageName::Implementation, &makefile_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
        "Implementation must NOT write build orchestration files (Makefile)"
    );
}

#[test]
fn path_traversal_double_dot_is_blocked() {
    let ws = workspace();
    let traversal_path = ws.join("src").join("..").join("etc").join("passwd");

    let result = validate_write_path(&StageName::Implementation, &traversal_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::PathTraversalDetected(_))),
        "Path traversal with '..' must be blocked"
    );
}

#[test]
fn path_traversal_absolute_escape_is_blocked() {
    let ws = workspace();
    let escape_path = PathBuf::from("/etc/passwd");

    let result = validate_write_path(&StageName::Implementation, &escape_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::NotWithinWorkspace { .. })),
        "Escape to absolute paths outside workspace must be blocked"
    );
}

#[test]
fn path_traversal_sibling_directory_is_blocked() {
    let ws = PathBuf::from("/home/user/project");
    let sibling_path = PathBuf::from("/home/user/project-malicious/payload.rs");

    let result = validate_write_path(&StageName::Implementation, &sibling_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::NotWithinWorkspace { .. })),
        "Sibling directories must not be accessible"
    );
}

#[test]
fn review_stage_rejects_all_writes() {
    let ws = workspace();

    let any_paths = vec![
        ws.join("src").join("lib.rs"),
        ws.join("tests").join("test.rs"),
        ws.join("docs").join("readme.md"),
        ws.join(".beads").join("config.json"),
    ];

    for path in any_paths {
        let result = validate_write_path(&StageName::Review, &path, &ws);
        assert!(
            matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
            "Review stage must be read-only for all paths: {:?}",
            path
        );
    }
}

#[test]
fn ship_gate_stage_accepts_beads_and_git() {
    let ws = workspace();

    let allowed_paths = vec![
        ws.join(".beads").join("src-14x.json"),
        ws.join(".beads").join("progress").join("src-14x.txt"),
        ws.join(".git").join("MERGE_HEAD"),
        ws.join(".git").join("refs").join("heads").join("main"),
    ];

    for path in allowed_paths {
        let result = validate_write_path(&StageName::ShipGate, &path, &ws);
        assert!(result.is_ok(), "ShipGate should allow .beads/ and .git/: {:?}", path);
    }
}

#[test]
fn ship_gate_stage_rejects_source_code() {
    let ws = workspace();
    let src_path = ws.join("src").join("main.rs");

    let result = validate_write_path(&StageName::ShipGate, &src_path, &ws);
    assert!(
        matches!(result, Err(WriteAllowlistError::WriteNotAllowed { .. })),
        "ShipGate must NOT write source code"
    );
}

#[test]
fn stage_write_config_contract_is_strict() {
    let config = StageWriteConfig::for_stage(StageName::Contract);

    assert!(!config.read_only, "Contract should not be read-only");
    assert!(
        config.allowed_dirs.contains(&PathBuf::from(".beads/contracts")),
        "Contract must allow .beads/contracts/"
    );
    assert!(
        config.allowed_patterns.iter().any(|p| p.contains("*.cue")),
        "Contract must allow *.cue pattern"
    );
}

#[test]
fn stage_write_config_implementation_excludes_orchestration() {
    let config = StageWriteConfig::for_stage(StageName::Implementation);

    assert!(!config.read_only);

    let disallowed = [".github", "scripts", ".moon", "Makefile", "Taskfile.yml", ".gitlab-ci.yml"];

    for disallow in disallowed {
        assert!(
            !config.allowed_dirs.iter().any(|d| d.to_string_lossy().contains(disallow)),
            "Implementation config must NOT allow {}: {:?}",
            disallow,
            config.allowed_dirs
        );
    }
}

#[test]
fn is_write_allowed_returns_false_for_contract_docs() {
    let ws = workspace();
    let docs_path = ws.join("docs").join("contract.md");

    assert!(
        !is_write_allowed(&StageName::Contract, &docs_path, &ws),
        "is_write_allowed must return false for Contract -> docs/"
    );
}

#[test]
fn validation_happens_before_persistence_check_order() {
    let ws = workspace();

    let bad_path = PathBuf::from("/home/user/project/../../../etc/passwd");

    let result = validate_write_path(&StageName::Implementation, &bad_path, &ws);

    assert!(
        matches!(result, Err(WriteAllowlistError::PathTraversalDetected(_))),
        "Path validation must occur before stage-specific checks"
    );
}

#[test]
fn empty_path_is_rejected() {
    let ws = workspace();
    let empty_path = Path::new("");

    let result = validate_write_path(&StageName::Implementation, empty_path, &ws);
    assert!(matches!(result, Err(WriteAllowlistError::EmptyPath)));
}

#[test]
fn control_characters_in_path_are_rejected() {
    let ws = workspace();
    let malicious_path = PathBuf::from("/home/user/project/src/main\u{0000}.rs");

    let result = validate_write_path(&StageName::Implementation, &malicious_path, &ws);
    assert!(matches!(result, Err(WriteAllowlistError::PathContainsControlChars)));
}

#[test]
fn relative_path_is_rejected() {
    let ws = workspace();
    let relative_path = Path::new("src/main.rs");

    let result = validate_write_path(&StageName::Implementation, relative_path, &ws);
    assert!(matches!(result, Err(WriteAllowlistError::RelativePath(_))));
}
