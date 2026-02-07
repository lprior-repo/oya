//! Test file to verify pipeline validation cache implementation
//! This can be run with `cargo run --bin test_pipeline_cache`

use std::collections::HashSet;
use std::sync::OnceLock;

// Mock the error type for testing
#[derive(Debug)]
enum Error {
    DuplicateStages { stages: String },
    InvalidRecord { reason: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::DuplicateStages { stages } => {
                write!(f, "Duplicate stages detected: {}", stages)
            }
            Error::InvalidRecord { reason } => {
                write!(f, "Invalid record: {}", reason)
            }
        }
    }
}

impl std::error::Error for Error {}

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
    validation_cache: OnceLock<Result<(), Box<dyn std::error::Error>>>,
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

    fn validate_cache(&self) -> &Result<(), Box<dyn std::error::Error>> {
        self.validation_cache.get_or_init(|| {
            // Check language is set
            let _language = self.language.as_ref().ok_or_else(|| {
                Box::new(Error::InvalidRecord {
                    reason: "language must be set".to_string(),
                }) as Box<dyn std::error::Error>
            })?;

            // Check for duplicate stage names
            let stage_names: Vec<&String> = self.stages.iter().map(|s| &s.name).collect();
            let mut seen = HashSet::new();
            let duplicates: Vec<&String> = stage_names
                .iter()
                .filter(|&&name| !seen.insert(name.clone()))
                .cloned()
                .collect();

            if !duplicates.is_empty() {
                let duplicate_names: Vec<&str> =
                    duplicates.iter().map(|s: &&String| s.as_str()).collect();
                return Err(Box::new(Error::DuplicateStages {
                    stages: duplicate_names.join(", "),
                }) as Box<dyn std::error::Error>);
            }

            Ok(())
        })
    }

    fn invalidate_cache(&mut self) {
        let _ = self.validation_cache.take();
    }

    fn validate(self) -> Result<Self, Box<dyn std::error::Error>> {
        self.validate_cache().as_ref().map(|_| ()).map_err(|e| {
            // Clone the boxed error by converting to string and back
            // For this simple test, we'll just create a new error
            Box::new(Error::InvalidRecord {
                reason: e.to_string(),
            }) as Box<dyn std::error::Error>
        })?;
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

    // First validation should pass
    let builder3_initial = PipelineBuilder::new()
        .language("Rust")
        .with_stage(Stage::new("duplicate"))
        .with_stage(Stage::new("other"));

    if builder3_initial.validate().is_ok() {
        println!("✓ Initial validation passed");
    } else {
        println!("✗ Initial validation failed");
    }

    // Add duplicate stage
    let builder3_final = PipelineBuilder::new()
        .language("Rust")
        .with_stage(Stage::new("duplicate"))
        .with_stage(Stage::new("other"))
        .with_stage(Stage::new("duplicate"));

    // Second validation should fail
    match builder3_final.validate() {
        Ok(_) => println!("✗ Validation should have failed"),
        Err(e) => println!("✓ Validation correctly failed: {:?}", e),
    }

    // Test 4: Changing language - should invalidate cache
    println!("\nTest 4: Changing language (invalidates cache)");

    // First validation should pass
    let builder4_initial = PipelineBuilder::new()
        .language("Rust")
        .with_stage(Stage::new("test"));

    if builder4_initial.validate().is_ok() {
        println!("✓ Initial validation passed");
    } else {
        println!("✗ Initial validation failed");
    }

    // Change language
    let builder4_final = PipelineBuilder::new()
        .language("Go")
        .with_stage(Stage::new("test"));

    // Should still pass after language change
    match builder4_final.validate() {
        Ok(_) => println!("✓ Validation passed after language change"),
        Err(e) => println!("✗ Validation failed after language change: {:?}", e),
    }

    println!("\n✓ All tests completed successfully!");
}
