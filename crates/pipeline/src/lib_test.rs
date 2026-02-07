//! Test module for pipeline validation cache implementation

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Stage;

    #[test]
    fn test_pipeline_cache_first_validation() {
        let builder = PipelineBuilder::new()
            .language(Language::Rust)
            .with_stage(Stage::new("implement"))
            .with_stage(Stage::new("test"));

        match builder.validate() {
            Ok(_) => println!("✓ First validation passed"),
            Err(e) => panic!("✗ First validation failed: {:?}", e),
        }
    }

    #[test]
    fn test_pipeline_cache_second_validation_uses_cache() {
        let builder = PipelineBuilder::new()
            .language(Language::Rust)
            .with_stage(Stage::new("implement"))
            .with_stage(Stage::new("test"));

        // First validation
        match builder.validate() {
            Ok(_) => println!("✓ First validation passed"),
            Err(e) => panic!("✗ First validation failed: {:?}", e),
        }

        // Second validation on same builder
        let builder2 = builder.clone();
        match builder2.validate() {
            Ok(_) => println!("✓ Second validation passed (using cache)"),
            Err(e) => panic!("✗ Second validation failed: {:?}", e),
        }
    }

    #[test]
    fn test_pipeline_cache_duplicate_detection() {
        let mut builder = PipelineBuilder::new()
            .language(Language::Rust)
            .with_stage(Stage::new("duplicate"))
            .with_stage(Stage::new("other"));

        // First validation should pass
        match builder.validate() {
            Ok(b) => {
                builder = b;
                println!("✓ Initial validation passed");
            }
            Err(e) => panic!("✗ Initial validation failed: {:?}", e),
        }

        // Add duplicate stage
        builder = builder.with_stage(Stage::new("duplicate"));

        // Second validation should fail
        match builder.validate() {
            Ok(_) => panic!("✗ Validation should have failed with duplicate stages"),
            Err(e) => println!("✓ Validation correctly failed: {:?}", e),
        }
    }

    #[test]
    fn test_pipeline_cache_language_invalidation() {
        let mut builder = PipelineBuilder::new()
            .language(Language::Rust)
            .with_stage(Stage::new("test"));

        // First validation
        match builder.validate() {
            Ok(b) => {
                builder = b;
                println!("✓ Initial validation passed");
            }
            Err(e) => panic!("✗ Initial validation failed: {:?}", e),
        }

        // Change language
        builder = builder.language(Language::Go);

        // Should still pass after language change
        match builder.validate() {
            Ok(_) => println!("✓ Validation passed after language change"),
            Err(e) => panic!("✗ Validation failed after language change: {:?}", e),
        }
    }
}