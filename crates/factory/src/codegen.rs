//! Code generation for functional Rust from bead requirements.
//!
//! Transforms bead specifications into functional Rust code that:
//! - Is immutable by default
//! - Uses pure functions
//! - Returns Result for all fallible operations
//! - Never panics
//! - Explicitly handles all errors

use crate::error::{Error, Result};
use crate::functional::{FunctionStub, Parameter, generate_functional_module};

/// Bead requirement specification.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BeadSpec {
    pub bead_id: String,
    pub description: String,
    pub functions: Vec<FunctionRequirement>,
    pub types: Vec<TypeRequirement>,
    pub tests: Vec<TestRequirement>,
}

/// Function requirement from a bead.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FunctionRequirement {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParamRequirement>,
    pub return_type: String,
    pub is_pure: bool,
}

/// Parameter requirement.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ParamRequirement {
    pub name: String,
    pub type_name: String,
    pub is_fallible: bool,
}

/// Type requirement.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TypeRequirement {
    pub name: String,
    pub fields: Vec<FieldRequirement>,
    pub is_opaque: bool,
}

/// Field in a type.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FieldRequirement {
    pub name: String,
    pub type_name: String,
    pub description: String,
}

/// Test requirement.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TestRequirement {
    pub function_name: String,
    pub description: String,
    pub input: Vec<String>,
    pub expected_output: String,
}

/// Generate functional Rust code from bead specification.
#[must_use]
pub fn generate_from_bead(spec: &BeadSpec) -> String {
    let mut output = String::new();

    // Module header
    output.push_str(&format!(
        "//! Module generated from bead {}\n",
        spec.bead_id
    ));
    output.push_str(&format!("//!\n"));
    output.push_str(&format!("//! {}\n", spec.description));
    output.push_str(&format!("//!\n"));
    output.push_str(&format!("//! Generated with functional Rust guarantees:\n"));
    output.push_str(&format!("//! - Immutable by default\n"));
    output.push_str(&format!("//! - Pure functions (no side effects)\n"));
    output.push_str(&format!("//! - Railway-Oriented Programming (Result)\n"));
    output.push_str(&format!("//! - Zero panics\n"));
    output.push_str(&format!("//! - Explicit error handling\n\n"));

    // Types
    output.push_str(
        "// =============================================================================\n",
    );
    output.push_str("// Types\n");
    output.push_str(
        "// =============================================================================\n\n",
    );
    for type_req in &spec.types {
        let type_code = generate_type(type_req);
        output.push_str(&type_code);
        output.push('\n');
    }

    // Functions
    output.push_str(
        "// =============================================================================\n",
    );
    output.push_str("// Functions (All Pure, Return Result for Fallible Ops)\n");
    output.push_str(
        "// =============================================================================\n\n",
    );

    let function_stubs: Vec<FunctionStub> = spec
        .functions
        .iter()
        .map(|f| FunctionStub {
            name: f.name.clone(),
            description: f.description.clone(),
            parameters: f
                .parameters
                .iter()
                .map(|p| Parameter {
                    name: p.name.clone(),
                    type_name: p.type_name.clone(),
                    is_result: p.is_fallible,
                })
                .collect(),
            return_type: f.return_type.clone(),
            returns_result: !f.is_pure,
        })
        .collect();

    output.push_str(&generate_functional_module(&spec.bead_id, &function_stubs));

    // Tests
    output.push_str(
        "// =============================================================================\n",
    );
    output.push_str("// Tests\n");
    output.push_str(
        "// =============================================================================\n\n",
    );

    output.push_str("#[cfg(test)]\n");
    output.push_str("mod tests {\n");
    output.push_str(&format!("    use super::*;\n\n"));

    for test_req in &spec.tests {
        output.push_str(&generate_test(test_req));
    }

    output.push_str("}\n");

    output
}

/// Generate a type definition.
#[must_use]
fn generate_type(type_req: &TypeRequirement) -> String {
    let mut output = String::new();

    if type_req.is_opaque {
        // Opaque validated type
        output.push_str(&format!("/// Validated {} type.\n", type_req.name));
        output.push_str(&format!(
            "/// Guarantees invariants are enforced at construction.\n"
        ));
        output.push_str("#[derive(Debug, Clone)]\n");
        output.push_str(&format!("pub struct {}(String);\n", type_req.name));

        output.push_str(&format!("impl {} {{\n", type_req.name));
        output.push_str(&format!(
            "    /// Create new {} with validation.\n",
            type_req.name
        ));
        output.push_str(&format!(
            "    pub fn new(value: impl Into<String>) -> Result<Self> {{\n"
        ));
        output.push_str(&format!("        let s = value.into();\n"));
        output.push_str(&format!("        // TODO: Add validation logic\n"));
        output.push_str(&format!("        Ok(Self(s))\n"));
        output.push_str(&format!("    }}\n\n"));

        output.push_str(&format!("    /// Get inner value.\n"));
        output.push_str("#[must_use]\n");
        output.push_str(&format!("    pub fn as_str(&self) -> &str {{\n"));
        output.push_str(&format!("        &self.0\n"));
        output.push_str(&format!("    }}\n"));
        output.push_str(&format!("}}\n"));
    } else {
        // Plain struct
        output.push_str(&format!("/// {}\n", type_req.name));
        output.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
        output.push_str(&format!("pub struct {} {{\n", type_req.name));

        for field in &type_req.fields {
            output.push_str(&format!("    /// {}\n", field.description));
            output.push_str(&format!("    pub {}: {},\n", field.name, field.type_name));
        }

        output.push_str("}\n");
    }

    output
}

/// Generate a test function.
#[must_use]
fn generate_test(test_req: &TestRequirement) -> String {
    let mut output = String::new();

    output.push_str(&format!("    #[test]\n"));
    output.push_str(&format!("    fn test_{}() {{\n", test_req.function_name));

    output.push_str(&format!("        // {}\n", test_req.description));

    output.push_str(&format!(
        "        // Input: {}\n",
        test_req.input.join(", ")
    ));

    output.push_str(&format!(
        "        // Expected: {}\n",
        test_req.expected_output
    ));

    output.push_str(&format!(
        "        // let result = {}({});\n",
        test_req.function_name,
        test_req.input.join(", ")
    ));

    output.push_str(&format!("        // assert!(result.is_ok());\n"));
    output.push_str(&format!("    }}\n\n"));

    output
}

/// Convert BeadSpec to a simple format for AI consumption.
#[must_use]
pub fn spec_to_prompt(spec: &BeadSpec) -> String {
    let mut prompt = String::new();

    prompt.push_str("# Functional Rust Implementation Requirements\n\n");
    prompt.push_str(&format!("## Bead: {}\n\n", spec.bead_id));
    prompt.push_str(&format!("{}\n\n", spec.description));

    prompt.push_str("## Types\n\n");
    for type_req in &spec.types {
        prompt.push_str(&format!("### `{}`\n", type_req.name));
        for field in &type_req.fields {
            prompt.push_str(&format!(
                "- `{}`: {} ({})\n",
                field.name, field.type_name, field.description
            ));
        }
        prompt.push('\n');
    }

    prompt.push_str("## Functions\n\n");
    for func in &spec.functions {
        prompt.push_str(&format!("### `{}: {}`\n", func.name, func.return_type));
        prompt.push_str(&format!("{}\n", func.description));
        prompt.push_str(&format!("- Pure: {}\n", func.is_pure));

        if !func.parameters.is_empty() {
            prompt.push_str("- Parameters:\n");
            for param in &func.parameters {
                prompt.push_str(&format!(
                    "  - `{}: {}`{}\n",
                    param.name,
                    param.type_name,
                    if param.is_fallible {
                        " (Result<T>)"
                    } else {
                        ""
                    }
                ));
            }
        }
        prompt.push('\n');
    }

    prompt.push_str("## Functional Rust Constraints\n\n");
    prompt.push_str("ALL generated code must follow these rules:\n\n");
    prompt.push_str("1. **Immutability**: Never use `mut` on variables or parameters\n");
    prompt.push_str("2. **No Panics**: Never use `panic!()`, `unwrap()`, or `expect()`\n");
    prompt.push_str("3. **Result Types**: All fallible operations return `Result<T, Error>`\n");
    prompt.push_str(
        "4. **Pattern Matching**: Use `match` instead of `unwrap()` for Options/Results\n",
    );
    prompt.push_str(
        "5. **Pure Functions**: Functions must not have side effects unless explicitly marked\n",
    );
    prompt.push_str("6. **Error Handling**: Explicitly handle all error cases with `?` operator\n");
    prompt.push_str(
        "7. **Zero Unwrap**: Code must pass `cargo clippy --no-deps -D clippy::unwrap_used`\n",
    );

    prompt.push('\n');

    prompt
}

/// Parse a bead spec from JSON string.
pub fn parse_bead_spec(json_str: &str) -> Result<BeadSpec> {
    serde_json::from_str(json_str)
        .map_err(|e| Error::json_parse_failed(format!("Failed to parse bead spec: {e}")))
}

/// Create a minimal bead spec for a simple function.
#[must_use]
pub fn simple_function_spec(bead_id: &str, function_name: &str, description: &str) -> BeadSpec {
    BeadSpec {
        bead_id: bead_id.to_string(),
        description: description.to_string(),
        functions: vec![FunctionRequirement {
            name: function_name.to_string(),
            description: description.to_string(),
            parameters: Vec::new(),
            return_type: "Result<i32>".to_string(),
            is_pure: false,
        }],
        types: Vec::new(),
        tests: Vec::new(),
    }
}

/// Validate that generated code follows functional patterns.
#[must_use]
pub fn validate_functional_code(code: &str) -> crate::functional::FunctionalAudit {
    crate::functional::audit_functional_style(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_from_bead() {
        let spec = BeadSpec {
            bead_id: "math-operations".to_string(),
            description: "Basic math operations".to_string(),
            functions: vec![FunctionRequirement {
                name: "add".to_string(),
                description: "Add two numbers".to_string(),
                parameters: vec![
                    ParamRequirement {
                        name: "a".to_string(),
                        type_name: "i32".to_string(),
                        is_fallible: false,
                    },
                    ParamRequirement {
                        name: "b".to_string(),
                        type_name: "i32".to_string(),
                        is_fallible: false,
                    },
                ],
                return_type: "i32".to_string(),
                is_pure: true,
            }],
            types: Vec::new(),
            tests: Vec::new(),
        };

        let code = generate_from_bead(&spec);
        assert!(code.contains("pub fn add"));
        assert!(code.contains("Functional Rust module"));
        assert!(code.contains("// TODO: Implement"));
    }

    #[test]
    fn test_spec_to_prompt() {
        let spec = simple_function_spec("test-bead", "calculate", "Calculate something");
        let prompt = spec_to_prompt(&spec);

        assert!(prompt.contains("Functional Rust Implementation Requirements"));
        assert!(prompt.contains("test-bead"));
        assert!(prompt.contains("calculate"));
        assert!(prompt.contains("Immutability"));
        assert!(prompt.contains("No Panics"));
    }

    #[test]
    fn test_parse_bead_spec() {
        let json = r#"{
            "bead_id": "test",
            "description": "Test",
            "functions": [{
                "name": "func",
                "description": "Test function",
                "parameters": [],
                "return_type": "Result<i32>",
                "is_pure": false
            }],
            "types": [],
            "tests": []
        }"#;

        let result = parse_bead_spec(json);
        assert!(result.is_ok());
        if let Ok(spec) = result {
            assert_eq!(spec.bead_id, "test");
            assert_eq!(spec.functions.len(), 1);
            assert_eq!(spec.functions[0].name, "func");
        }
    }

    #[test]
    fn test_simple_function_spec() {
        let spec = simple_function_spec("simple", "do_thing", "Do a thing");
        assert_eq!(spec.bead_id, "simple");
        assert_eq!(spec.functions.len(), 1);
        assert_eq!(spec.functions[0].name, "do_thing");
    }

    #[test]
    fn test_validate_functional_code() {
        let good_code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
        let bad_code = "pub fn get() -> i32 { val.unwrap() }";

        let good_audit = validate_functional_code(good_code);
        let bad_audit = validate_functional_code(bad_code);

        assert!(good_audit.is_functional());
        assert!(!bad_audit.is_functional());
    }
}
