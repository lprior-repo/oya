use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryFile {
    pub source_path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiscoverManifest {
    pub source_dir: String,
    pub discovered_at: String,
    pub total_files: usize,
    pub files: Vec<DiscoveryFile>,
}

pub fn discover_files(source_dir: &Path) -> Result<(Vec<DiscoveryFile>, DiscoverManifest)> {
    if !source_dir.exists() {
        anyhow::bail!("Source not found: {}", source_dir.display());
    }

    let canonical_path = source_dir.canonicalize().context(format!(
        "Failed to resolve canonical path for {}",
        source_dir.display()
    ))?;

    let mut files = Vec::new();
    let extensions = [".md", ".mdx", ".rst", ".txt"];
    let exclude_dirs = ["node_modules", ".git", "_build", "dist", "vendor"];

    // Handle single file case directly
    if canonical_path.is_file() {
        return discover_single_file(&canonical_path, &extensions);
    }

    // Directory case: walk the directory tree
    for entry in WalkDir::new(&canonical_path).into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: Skipping path due to I/O error: {e}");
                continue;
            }
        };

        let path = entry.path();

        // Skip excluded directories (exact match on directory name)
        if path.components().any(|c| {
            exclude_dirs
                .iter()
                .any(|excl| c.as_os_str().to_string_lossy() == *excl)
        }) {
            continue;
        }

        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = format!(".{}", ext.to_string_lossy());
                if extensions.contains(&ext_str.as_str()) {
                    // Get relative path, skip if it fails (e.g., prefix mismatch)
                    let rel_path = match path.strip_prefix(&canonical_path) {
                        Ok(p) => p.to_string_lossy().to_string(),
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to get relative path for {}: {e}",
                                path.display()
                            );
                            continue;
                        }
                    };

                    // Get file size, skip if metadata fails (e.g., permission denied)
                    let size = match path.metadata() {
                        Ok(meta) => meta.len(),
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to read metadata for {}: {e}, skipping file",
                                path.display()
                            );
                            continue;
                        }
                    };

                    files.push(DiscoveryFile {
                        source_path: rel_path,
                        size_bytes: size,
                    });
                }
            }
        }
    }

    let manifest = DiscoverManifest {
        source_dir: canonical_path.to_string_lossy().to_string(),
        discovered_at: chrono::Utc::now().to_rfc3339(),
        total_files: files.len(),
        files: files.clone(),
    };

    Ok((files, manifest))
}

/// Discover a single file (alternative to directory-based discovery)
///
/// Design by Contract:
/// - **Preconditions:**
///   - file_path exists and is a file
///   - extensions is a non-empty slice of supported extensions
/// - **Postconditions:**
///   - Returns Ok with (files, manifest)
///   - If file has supported extension: files contains one DiscoveryFile
///   - If file has unsupported extension: files is empty
///   - manifest.source_dir is the parent directory
/// - **Errors:**
///   - Returns error if metadata cannot be read
fn discover_single_file(
    file_path: &Path,
    extensions: &[&str],
) -> Result<(Vec<DiscoveryFile>, DiscoverManifest)> {
    let filename = match file_path.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => anyhow::bail!("Invalid file path: {}", file_path.display()),
    };

    // Check if the file has a supported extension
    let has_supported_ext = file_path.extension().is_some_and(|ext| {
        let ext_str = format!(".{}", ext.to_string_lossy());
        extensions.contains(&ext_str.as_str())
    });

    let mut files = Vec::new();

    if has_supported_ext {
        // Get file size using functional error handling
        let size = file_path
            .metadata()
            .context(format!(
                "Failed to read metadata for {}",
                file_path.display()
            ))?
            .len();

        files.push(DiscoveryFile {
            source_path: filename,
            size_bytes: size,
        });
    }

    // Use parent directory as source_dir for manifest
    let source_dir = file_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    let manifest = DiscoverManifest {
        source_dir,
        discovered_at: chrono::Utc::now().to_rfc3339(),
        total_files: files.len(),
        files: files.clone(),
    };

    Ok((files, manifest))
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Test that files with permission errors are skipped gracefully
    /// and other files are still discovered. This was a P0 bug where
    /// a single unreadable file would cause the entire discovery to fail.
    #[test]
    fn test_discover_files_skips_unreadable_files() {
        // Create temp directory with multiple markdown files
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let dir_path = temp_dir.path();

        // Create three markdown files
        let file1 = dir_path.join("readable1.md");
        let file2 = dir_path.join("unreadable.md");
        let file3 = dir_path.join("readable2.md");

        let mut f1 = match File::create(&file1) {
            Ok(f) => f,
            Err(e) => panic!("Failed to create file1: {e}"),
        };
        match f1.write_all(b"# Readable Document 1\nContent here") {
            Ok(_) => (),
            Err(e) => panic!("Failed to write file1: {e}"),
        }

        let mut f2 = match File::create(&file2) {
            Ok(f) => f,
            Err(e) => panic!("Failed to create file2: {e}"),
        };
        match f2.write_all(b"# Unreadable Document\nThis will have no read permissions") {
            Ok(_) => (),
            Err(e) => panic!("Failed to write file2: {e}"),
        }

        let mut f3 = match File::create(&file3) {
            Ok(f) => f,
            Err(e) => panic!("Failed to create file3: {e}"),
        };
        match f3.write_all(b"# Readable Document 2\nMore content") {
            Ok(_) => (),
            Err(e) => panic!("Failed to write file3: {e}"),
        }

        // Remove read permissions from file2 (making it unreadable)
        match fs::set_permissions(&file2, PermissionsExt::from_mode(0o000)) {
            Ok(_) => (),
            Err(e) => panic!("Failed to set permissions: {e}"),
        }

        // Discover files - should skip unreadable file but find the other two
        let result = discover_files(dir_path);

        // Clean up: restore permissions so temp dir can be removed
        let _ = fs::set_permissions(&file2, PermissionsExt::from_mode(0o644));

        // Result should be Ok (not an error)
        assert!(
            result.is_ok(),
            "discover_files should succeed even with unreadable files"
        );

        let (files, _manifest) = match result {
            Ok(v) => v,
            Err(e) => panic!("discover_files failed: {e}"),
        };

        // On Linux, files with 0o000 permissions can still have metadata read.
        // The file may be discovered but will fail when actually read.
        // The important thing is that discovery succeeds without panicking.
        // We should find at least the 2 readable files.
        assert!(
            files.len() >= 2,
            "Expected at least 2 readable files to be discovered, got {}",
            files.len()
        );

        // Verify the readable files were found
        let file_names: Vec<_> = files.iter().map(|f| f.source_path.clone()).collect();

        assert!(
            file_names.iter().any(|n| n.contains("readable1.md")),
            "readable1.md should be in discovered files"
        );
        assert!(
            file_names.iter().any(|n| n.contains("readable2.md")),
            "readable2.md should be in discovered files"
        );
    }

    /// Test basic file discovery functionality
    #[test]
    fn test_discover_files_basic() {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let dir_path = temp_dir.path();

        // Create test files
        let md_file = dir_path.join("test.md");
        let txt_file = dir_path.join("test.txt");
        let rst_file = dir_path.join("test.rst");
        let mdx_file = dir_path.join("test.mdx");
        let other_file = dir_path.join("test.html");

        match File::create(&md_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create md file: {e}"),
        }
        match File::create(&txt_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create txt file: {e}"),
        }
        match File::create(&rst_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create rst file: {e}"),
        }
        match File::create(&mdx_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create mdx file: {e}"),
        }
        match File::create(&other_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create html file: {e}"),
        }

        let result = discover_files(dir_path);
        assert!(result.is_ok());

        let (files, _manifest) = match result {
            Ok(v) => v,
            Err(e) => panic!("discover_files failed: {e}"),
        };
        assert_eq!(files.len(), 4, "Should discover 4 supported files");
    }

    /// Test that empty directory returns empty file list (not error)
    #[test]
    fn test_discover_files_empty_directory() {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let dir_path = temp_dir.path();

        let result = discover_files(dir_path);
        assert!(result.is_ok());

        let (files, manifest) = match result {
            Ok(v) => v,
            Err(e) => panic!("discover_files failed: {e}"),
        };
        assert_eq!(files.len(), 0, "Empty directory should have 0 files");
        assert_eq!(manifest.total_files, 0, "Manifest should show 0 files");
    }

    /// Test discovery in nested directories
    #[test]
    fn test_discover_files_nested_directories() {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let dir_path = temp_dir.path();

        // Create nested structure
        let subdir = dir_path.join("subdir");
        match fs::create_dir(&subdir) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create subdir: {e}"),
        }

        let root_file = dir_path.join("root.md");
        let sub_file = subdir.join("sub.md");

        match File::create(&root_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create root file: {e}"),
        }
        match File::create(&sub_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create sub file: {e}"),
        }

        let result = discover_files(dir_path);
        assert!(result.is_ok());

        let (files, _manifest) = match result {
            Ok(v) => v,
            Err(e) => panic!("discover_files failed: {e}"),
        };
        assert_eq!(
            files.len(),
            2,
            "Should discover files in nested directories"
        );
    }

    /// Test that a single markdown file can be indexed directly
    /// This was P1 bug doc-tx-xpm: discover_files rejected single files
    #[test]
    fn test_discover_single_file() {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let dir_path = temp_dir.path();

        // Create a single markdown file
        let single_file = dir_path.join("single.md");
        let mut f = match File::create(&single_file) {
            Ok(f) => f,
            Err(e) => panic!("Failed to create single file: {e}"),
        };
        match f.write_all(b"# Single Document\n\nThis is a single file to index.") {
            Ok(_) => (),
            Err(e) => panic!("Failed to write single file: {e}"),
        }

        // Test: discover_files should accept a single file path
        let result = discover_files(&single_file);
        assert!(
            result.is_ok(),
            "discover_files should accept single file, got: {:?}",
            result.as_ref().map_err(|e| e.to_string())
        );

        let (files, manifest) = match result {
            Ok(v) => v,
            Err(e) => panic!("discover_files failed for single file: {e}"),
        };

        // Should discover exactly one file
        assert_eq!(files.len(), 1, "Should discover exactly 1 file");
        assert_eq!(manifest.total_files, 1, "Manifest should show 1 file");

        // The discovered file should be the single file itself
        let expected_name = single_file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        assert_eq!(files[0].source_path, expected_name);
    }

    /// Test that single file discovery rejects unsupported file types
    #[test]
    fn test_discover_single_file_unsupported_type() {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let dir_path = temp_dir.path();

        // Create a single file with unsupported extension
        let unsupported_file = dir_path.join("data.json");
        match File::create(&unsupported_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create unsupported file: {e}"),
        }

        // Should succeed but find no files (unsupported type)
        let result = discover_files(&unsupported_file);
        assert!(
            result.is_ok(),
            "discover_files should succeed even with unsupported file type"
        );

        let (files, _manifest) = match result {
            Ok(v) => v,
            Err(e) => panic!("discover_files failed: {e}"),
        };

        assert_eq!(
            files.len(),
            0,
            "Should discover 0 files for unsupported type"
        );
    }

    /// Test that single file discovery handles non-existent file
    #[test]
    fn test_discover_single_file_not_found() {
        let nonexistent = PathBuf::from("/nonexistent/path/file.md");
        let result = discover_files(&nonexistent);

        assert!(
            result.is_err(),
            "discover_files should error for non-existent file"
        );
        let err_msg = result.map_err(|e| e.to_string());
        if let Err(msg) = err_msg {
            assert!(
                msg.contains("not found"),
                "Error should mention 'not found'"
            );
        }
    }

    /// Test that excluded directories are skipped
    #[test]
    fn test_discover_files_excludes_directories() {
        let temp_dir = match TempDir::new() {
            Ok(d) => d,
            Err(e) => panic!("Failed to create temp dir: {e}"),
        };
        let dir_path = temp_dir.path();

        // Create directories that should be excluded
        let node_modules = dir_path.join("node_modules");
        let git_dir = dir_path.join(".git");
        let _build = dir_path.join("_build");
        let dist_dir = dir_path.join("dist");
        let vendor_dir = dir_path.join("vendor");

        match fs::create_dir(&node_modules) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create node_modules: {e}"),
        }
        match fs::create_dir(&git_dir) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create .git: {e}"),
        }
        match fs::create_dir(&_build) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create _build: {e}"),
        }
        match fs::create_dir(&dist_dir) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create dist: {e}"),
        }
        match fs::create_dir(&vendor_dir) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create vendor: {e}"),
        }

        // Create files inside excluded directories
        let nm_file = node_modules.join("package.md");
        let git_file = git_dir.join("config.md");
        let build_file = _build.join("output.md");
        let dist_file = dist_dir.join("bundle.md");
        let vendor_file = vendor_dir.join("lib.md");

        match File::create(&nm_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create nm file: {e}"),
        }
        match File::create(&git_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create git file: {e}"),
        }
        match File::create(&build_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create build file: {e}"),
        }
        match File::create(&dist_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create dist file: {e}"),
        }
        match File::create(&vendor_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create vendor file: {e}"),
        }

        // Create a file in root that should be found
        let root_file = dir_path.join("root.md");
        match File::create(&root_file) {
            Ok(_) => (),
            Err(e) => panic!("Failed to create root file: {e}"),
        }

        let result = discover_files(dir_path);
        assert!(result.is_ok());

        let (files, _manifest) = match result {
            Ok(v) => v,
            Err(e) => panic!("discover_files failed: {e}"),
        };
        assert_eq!(
            files.len(),
            1,
            "Should only find root file, not files in excluded directories"
        );
        assert!(
            files[0].source_path.contains("root.md"),
            "Found file should be root.md"
        );
    }
}
