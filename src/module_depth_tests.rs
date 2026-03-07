#[cfg(test)]
mod module_depth_tests {
    use std::fs;
    use std::path::Path;

    const MAX_MODULE_DEPTH: usize = 3;

    fn get_all_rust_files() -> Vec<String> {
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
                    collect_rust_files_recursive(&path, &mut files);
                }
            }
        }
        files
    }

    fn collect_rust_files_recursive(dir: &Path, files: &mut Vec<String>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "rs") {
                    if let Some(path_str) = path.to_str() {
                        files.push(path_str.to_string());
                    }
                }
                if path.is_dir() {
                    collect_rust_files_recursive(&path, files);
                }
            }
        }
    }

    fn calculate_module_depth(file_path: &str) -> Result<usize, String> {
        let path = Path::new(file_path);
        
        let components: Vec<_> = path
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        let src_index = components
            .iter()
            .position(|c| *c == "src")
            .ok_or_else(|| format!("File {} is not under src/", file_path))?;

        let depth = components.len() - src_index - 2;

        Ok(depth)
    }

    fn is_test_file(path: &str) -> bool {
        let filename = Path::new(path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        
        filename.ends_with("_tests.rs") || filename == "tests.rs"
    }

    #[test]
    fn test_no_module_exceeds_max_depth() {
        let files = get_all_rust_files();
        let mut violations: Vec<String> = Vec::new();

        for file in files {
            if is_test_file(&file) {
                continue;
            }

            match calculate_module_depth(&file) {
                Ok(depth) => {
                    if depth > MAX_MODULE_DEPTH {
                        violations.push(format!(
                            "{}: depth {} exceeds maximum allowed depth of {}",
                            file, depth, MAX_MODULE_DEPTH
                        ));
                    }
                }
                Err(e) => {
                    violations.push(format!("{}: {}", file, e));
                }
            }
        }

        if !violations.is_empty() {
            let mut report = String::new();
            report.push_str(&format!(
                "\n=== Module Depth Violations (max allowed: {}) ===\n",
                MAX_MODULE_DEPTH
            ));
            report.push_str(&format!("Found {} violations:\n\n", violations.len()));
            
            for violation in &violations {
                report.push_str(&format!("  {}\n", violation));
            }
            
            report.push_str("\nHint: Flatten deep module hierarchies. Consider:\n");
            report.push_str("  - Merging subdirectories into parent modules\n");
            report.push_str("  - Renaming files to be more descriptive at shallower levels\n");
            report.push_str("  - Using module re-exports in mod.rs to simplify structure\n");

            panic!("{}", report);
        }
    }

    #[test]
    fn test_module_depth_calculation() {
        assert_eq!(calculate_module_depth("src/lib.rs").unwrap(), 0);
        assert_eq!(calculate_module_depth("src/domain/mod.rs").unwrap(), 1);
        assert_eq!(calculate_module_depth("src/lifecycle/types/bead.rs").unwrap(), 2);
        assert_eq!(calculate_module_depth("src/cli/doctor/commands.rs").unwrap(), 3);
    }

    #[test]
    fn test_current_structure_compliance() {
        let files = get_all_rust_files();
        let compliant_count = files
            .iter()
            .filter(|f| !is_test_file(f))
            .filter(|f| calculate_module_depth(f).map(|d| d <= MAX_MODULE_DEPTH).unwrap_or(false))
            .count();

        let total_count = files.iter().filter(|f| !is_test_file(f)).count();

        println!(
            "Module depth compliance: {}/{} files ({:.1}%) are within depth limit",
            compliant_count,
            total_count,
            if total_count > 0 {
                (compliant_count as f64 / total_count as f64) * 100.0
            } else {
                0.0
            }
        );
    }
}
