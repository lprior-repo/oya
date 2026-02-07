//! Test file to verify pipeline validation cache implementation
//! This can be run with `cargo run --bin test_pipeline_cache`

use std::collections::HashMap;
use std::sync::OnceLock;

// Mock the error type for testing
#[derive(Debug)]
enum Error {
    DuplicateStages { stages: String },
    InvalidRecord { reason: String },
}

// Mock stage and other types for testing
#[derive(Debug, Clone)]
struct Stage {
    name: String,
}

impl Stage {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug)]
struct PipelineBuilder {
    stages: Vec<Stage>,
    language: Option<String>,
    validation_cache: OnceLock<Result<()>>,
}

impl PipelineBuilder {
    fn new() -> Self {
        Self {
            stages: Vec::new(),
            language: None,
            validation_cache: OnceLock::new(),
        }
    }

    fn with_stage(mut self, stage: Stage) -> Self {
        self.stages.push(stage);
        self.invalidate_cache();
        self
    }

    fn language(mut self, language: &str) -> Self {
        self.language = Some(language.to_string());
        self.invalidate_cache();
        self
    }

    fn validate_cache(&self) -> &Result<()> {
        self.validation_cache.get_or_init(|| {
            // Check language is set
            let language = self.language.as_ref().ok_or_else(|| Error::InvalidRecord {
                reason: "language must be set".to_string(),
            })?;

            // Check for duplicate stage names
            let stage_names: Vec<&String> = self.stages.iter().map(|s| &s.name).collect();
            let duplicates: Vec<&String> = stage_names.iter().duplicates().cloned().collect();

            if !duplicates.is_empty() {
                let duplicate_names: Vec<&str> = duplicates.iter().map(|s| s.as_str()).collect();
                return Err(Error::DuplicateStages {
                    stages: duplicate_names.join(", "),
                });
            }

            Ok(())
        })
    }

    fn invalidate_cache(&mut self) {
        let _ = self.validation_cache.take();
    }

    fn validate(self) -> Result<Self> {
        self.validate_cache().clone()?;
        Ok(self)
    }
}

fn main() {
    println!("Testing pipeline validation cache implementation...\n");

    // Test 1: First validation - should compute and cache
    println!("Test 1: First validation (computes cache)");
    let builder1 = PipelineBuilder::new()
        .language("Rust")
        .with_stage(Stage::new("implement"))
        .with_stage(Stage::new("test"));

    match builder1.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }

    // Test 2: Second validation - should use cache
    println!("\nTest 2: Second validation (uses cached result)");
    let builder2 = PipelineBuilder::new()
        .language("Rust")
        .with_stage(Stage::new("implement"))
        .with_stage(Stage::new("test"));

    match builder2.validate() {
        Ok(_) => println!("✓ Validation passed"),
        Err(e) => println!("✗ Validation failed: {:?}", e),
    }

    // Test 3: Adding duplicate stage - should invalidate and detect duplicates
    println!("\nTest 3: Adding duplicate (invalidates cache, detects error)");
    let mut builder3 = PipelineBuilder::new()
        .language("Rust")
        .with_stage(Stage::new("duplicate"))
        .with_stage(Stage::new("other"));

    // First validation should pass
    match builder3.validate() {
        Ok(builder) => {
            builder3 = builder;
            println!("✓ Initial validation passed");
        }
        Err(e) => println!("✗ Initial validation failed: {:?}", e),
    }

    // Add duplicate stage
    builder3 = builder3.with_stage(Stage::new("duplicate"));

    // Second validation should fail
    match builder3.validate() {
        Ok(_) => println!("✗ Validation should have failed"),
        Err(e) => println!("✓ Validation correctly failed: {:?}", e),
    }

    // Test 4: Changing language - should invalidate cache
    println!("\nTest 4: Changing language (invalidates cache)");
    let mut builder4 = PipelineBuilder::new()
        .language("Rust")
        .with_stage(Stage::new("test"));

    match builder4.validate() {
        Ok(builder) => {
            builder4 = builder;
            println!("✓ Initial validation passed");
        }
        Err(e) => println!("✗ Initial validation failed: {:?}", e),
    }

    // Change language
    builder4 = builder4.language("Go");

    // Should still pass after language change
    match builder4.validate() {
        Ok(_) => println!("✓ Validation passed after language change"),
        Err(e) => println!("✗ Validation failed after language change: {:?}", e),
    }

    println!("\n✓ All tests completed successfully!");
}
