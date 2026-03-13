#[cfg(test)]
mod zero_panic_lint_tests {
    use std::fs;
    use std::path::Path;

    fn get_rust_source_files() -> Vec<String> {
        let mut files = Vec::new();
        let src_path = Path::new("src");

        if let Ok(entries) = fs::read_dir(src_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "rs") {
                    if let Some(path_str) = path.to_str() {
                        files.push(path_str.to_string());
                    }
                }
                if path.is_dir() {
                    collect_rust_files(&path, &mut files);
                }
            }
        }
        files
    }

    fn collect_rust_files(dir: &Path, files: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "rs") {
                    if let Some(path_str) = path.to_str() {
                        if !path_str.ends_with("tests.rs") && !path_str.ends_with("_tests.rs") {
                            files.push(path_str.to_string());
                        }
                    }
                }
                if path.is_dir() {
                    collect_rust_files(&path, files);
                }
            }
        }
    }

    fn has_lint_attribute(content: &str, lint_name: &str) -> bool {
        content.contains(&format!("#![deny(clippy::{})]", lint_name))
            || content.contains(&format!("#![forbid(clippy::{})]", lint_name))
    }

    fn is_test_file(path: &str) -> bool {
        path.ends_with("tests.rs") || path.ends_with("_tests.rs")
    }

    #[test]
    fn test_all_source_files_have_unwrap_deny() {
        let files = get_rust_source_files();
        let mut violations = Vec::new();

        for file in files {
            if is_test_file(&file) {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&file) {
                if !has_lint_attribute(&content, "unwrap_used") {
                    violations.push(format!("{}: missing #![deny(clippy::unwrap_used)]", file));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Source files missing unwrap_used lint:\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn test_all_source_files_have_expect_deny() {
        let files = get_rust_source_files();
        let mut violations = Vec::new();

        for file in files {
            if is_test_file(&file) {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&file) {
                if !has_lint_attribute(&content, "expect_used") {
                    violations.push(format!("{}: missing #![deny(clippy::expect_used)]", file));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Source files missing expect_used lint:\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn test_all_source_files_have_panic_deny() {
        let files = get_rust_source_files();
        let mut violations = Vec::new();

        for file in files {
            if is_test_file(&file) {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&file) {
                if !has_lint_attribute(&content, "panic") {
                    violations.push(format!("{}: missing #![deny(clippy::panic)]", file));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Source files missing panic lint:\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn test_main_has_unsafe_code_forbid() {
        let main_path = "src/main.rs";
        if let Ok(content) = fs::read_to_string(main_path) {
            assert!(content.contains("#![forbid(unsafe_code)]"), "main.rs must forbid unsafe code");
        } else {
            panic!("Could not read main.rs");
        }
    }

    #[test]
    fn test_no_unwrap_in_source_files() {
        let files = get_rust_source_files();
        let mut violations = Vec::new();

        for file in files {
            if is_test_file(&file) {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&file) {
                for (line_num, line) in content.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                        continue;
                    }

                    if trimmed.contains(".unwrap()") && !trimmed.starts_with("//") {
                        violations.push(format!(
                            "{}:{}: found .unwrap() - {}",
                            file,
                            line_num + 1,
                            trimmed
                        ));
                    }
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Source files must not contain .unwrap():\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn test_no_expect_in_source_files() {
        let files = get_rust_source_files();
        let mut violations = Vec::new();

        for file in files {
            if is_test_file(&file) {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&file) {
                for (line_num, line) in content.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                        continue;
                    }

                    if trimmed.contains(".expect(") && !trimmed.starts_with("//") {
                        violations.push(format!(
                            "{}:{}: found .expect() - {}",
                            file,
                            line_num + 1,
                            trimmed
                        ));
                    }
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Source files must not contain .expect():\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn test_no_panic_macro_in_source_files() {
        let files = get_rust_source_files();
        let mut violations = Vec::new();

        for file in files {
            if is_test_file(&file) {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&file) {
                for (line_num, line) in content.lines().enumerate() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                        continue;
                    }

                    if (trimmed.contains("panic!(")
                        || trimmed.contains("todo!(")
                        || trimmed.contains("unimplemented!("))
                        && !trimmed.starts_with("//")
                    {
                        violations.push(format!(
                            "{}:{}: found panic-like macro - {}",
                            file,
                            line_num + 1,
                            trimmed
                        ));
                    }
                }
            }
        }

        assert!(
            violations.is_empty(),
            "Source files must not contain panic!/todo!/unimplemented!():\n{}",
            violations.join("\n")
        );
    }

    #[test]
    fn test_domain_module_has_lints() {
        let domain_files = vec!["src/domain/mod.rs", "src/domain/types.rs", "src/domain/bead.rs"];

        for file in domain_files {
            if Path::new(file).exists() {
                if let Ok(content) = fs::read_to_string(file) {
                    assert!(
                        has_lint_attribute(&content, "unwrap_used"),
                        "{} must have #![deny(clippy::unwrap_used)]",
                        file
                    );
                    assert!(
                        has_lint_attribute(&content, "expect_used"),
                        "{} must have #![deny(clippy::expect_used)]",
                        file
                    );
                    assert!(
                        has_lint_attribute(&content, "panic"),
                        "{} must have #![deny(clippy::panic)]",
                        file
                    );
                }
            }
        }
    }

    #[test]
    fn test_lattice_module_has_lints() {
        let lattice_files =
            vec!["src/lattice/mod.rs", "src/lattice/coordinator.rs", "src/lattice/execution.rs"];

        for file in lattice_files {
            if Path::new(file).exists() {
                if let Ok(content) = fs::read_to_string(file) {
                    assert!(
                        has_lint_attribute(&content, "unwrap_used"),
                        "{} must have #![deny(clippy::unwrap_used)]",
                        file
                    );
                    assert!(
                        has_lint_attribute(&content, "expect_used"),
                        "{} must have #![deny(clippy::expect_used)]",
                        file
                    );
                    assert!(
                        has_lint_attribute(&content, "panic"),
                        "{} must have #![deny(clippy::panic)]",
                        file
                    );
                }
            }
        }
    }

    #[test]
    fn test_intent_module_has_lints() {
        let intent_files =
            vec!["src/intent/mod.rs", "src/intent/parser.rs", "src/intent/executor.rs"];

        for file in intent_files {
            if Path::new(file).exists() {
                if let Ok(content) = fs::read_to_string(file) {
                    assert!(
                        has_lint_attribute(&content, "unwrap_used"),
                        "{} must have #![deny(clippy::unwrap_used)]",
                        file
                    );
                    assert!(
                        has_lint_attribute(&content, "expect_used"),
                        "{} must have #![deny(clippy::expect_used)]",
                        file
                    );
                    assert!(
                        has_lint_attribute(&content, "panic"),
                        "{} must have #![deny(clippy::panic)]",
                        file
                    );
                }
            }
        }
    }

    #[test]
    fn test_clippy_config_exists() {
        assert!(Path::new(".clippy.toml").exists(), "Project must have .clippy.toml configuration");
    }

    #[test]
    fn test_result_type_used_for_fallible_operations() {
        let files = get_rust_source_files();
        let mut warnings = Vec::new();

        for file in files {
            if is_test_file(&file) {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&file) {
                if content.contains("pub fn ") || content.contains("pub async fn ") {
                    for line in content.lines() {
                        let trimmed = line.trim();
                        if (trimmed.starts_with("pub fn ") || trimmed.starts_with("pub async fn "))
                            && !trimmed.contains("Result<")
                            && !trimmed.starts_with("//")
                        {
                            if trimmed.contains("-> ") && !trimmed.contains("()") {
                                warnings.push(format!(
                                    "{}: consider using Result for fallible function",
                                    file
                                ));
                            }
                        }
                    }
                }
            }
        }

        if !warnings.is_empty() {
            eprintln!("Warning: Some functions may need Result<T, E>:\n{}", warnings.join("\n"));
        }
    }

    #[test]
    fn test_error_types_define_proper_variants() {
        let error_files =
            vec!["src/error.rs", "src/errors.rs", "src/domain/error.rs", "src/lattice/error.rs"];

        let mut found_error_file = false;
        for file in error_files {
            if Path::new(file).exists() {
                found_error_file = true;
                if let Ok(content) = fs::read_to_string(file) {
                    assert!(
                        content.contains("enum") || content.contains("struct"),
                        "{} must define error types",
                        file
                    );
                }
            }
        }

        if !found_error_file {
            eprintln!("Warning: No dedicated error type file found");
        }
    }
}
