use oya_pipeline::file_discovery::{
    cache_stats, clear_cache, find_javascript_files, find_python_files, find_rust_files,
};
use std::path::Path;

fn main() {
    println!("Testing file discovery memoization...");

    // Create a test directory structure
    let test_dir = std::env::temp_dir().join("oya_memoization_test");
    std::fs::create_dir_all(&test_dir).unwrap();

    // Create test files
    std::fs::write(test_dir.join("main.rs"), "fn main() {}").unwrap();
    std::fs::write(test_dir.join("lib.rs"), "pub fn lib() {}").unwrap();
    std::fs::write(test_dir.join("app.py"), "print('hello')").unwrap();
    std::fs::write(test_dir.join("main.py"), "def main(): pass").unwrap();
    std::fs::write(test_dir.join("app.js"), "console.log('hello');").unwrap();
    std::fs::write(test_dir.join("app.ts"), "const x = 1;").unwrap();

    // Create subdirectory
    let sub_dir = test_dir.join("subdir");
    std::fs::create_dir_all(&sub_dir).unwrap();
    std::fs::write(sub_dir.join("helper.rs"), "pub fn helper() {}").unwrap();
    std::fs::write(sub_dir.join("tool.py"), "def tool(): pass").unwrap();
    std::fs::write(sub_dir.join("util.js"), "function util() {}").unwrap();

    println!("Created test directory: {}", test_dir.display());

    // First call - should populate cache
    println!("\n=== First call (cache miss) ===");
    let rust_files1 = find_rust_files(&test_dir).unwrap();
    println!("Found {} Rust files", rust_files1.len());

    let python_files1 = find_python_files(&test_dir).unwrap();
    println!("Found {} Python files", python_files1.len());

    let js_files1 = find_javascript_files(&test_dir).unwrap();
    println!("Found {} JavaScript/TypeScript files", js_files1.len());

    let (entries1, total_files1) = cache_stats();
    println!("Cache entries: {}, Total files: {}", entries1, total_files1);

    // Second call - should use cache
    println!("\n=== Second call (cache hit) ===");
    let rust_files2 = find_rust_files(&test_dir).unwrap();
    println!("Found {} Rust files", rust_files2.len());

    let python_files2 = find_python_files(&test_dir).unwrap();
    println!("Found {} Python files", python_files2.len());

    let js_files2 = find_javascript_files(&test_dir).unwrap();
    println!("Found {} JavaScript/TypeScript files", js_files2.len());

    let (entries2, total_files2) = cache_stats();
    println!("Cache entries: {}, Total files: {}", entries2, total_files2);

    // Verify results are the same
    assert_eq!(rust_files1.len(), rust_files2.len());
    assert_eq!(python_files1.len(), python_files2.len());
    assert_eq!(js_files1.len(), js_files2.len());

    assert_eq!(entries1, entries2);
    assert_eq!(total_files1, total_files2);

    println!("\n=== Cache management ===");
    println!("Before clear: {} entries, {} files", entries2, total_files2);
    clear_cache();
    let (entries3, total_files3) = cache_stats();
    println!("After clear: {} entries, {} files", entries3, total_files3);

    assert_eq!(entries3, 0);
    assert_eq!(total_files3, 0);

    println!("\n=== Test completed successfully! ===");

    // Cleanup
    std::fs::remove_dir_all(&test_dir).unwrap();
}
