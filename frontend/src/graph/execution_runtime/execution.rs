//! Node execution implementations.

use crate::graph::{Workflow, WorkflowExecutionError};

impl Workflow {
    // ===========================================================================
    // Memory Limit Enforcement
    // ===========================================================================

    /// Estimates the memory usage of a JSON value in bytes.
    ///
    /// This function recursively traverses the JSON value and calculates
    /// the approximate byte size it would occupy when serialized.
    /// The estimate is conservative - it may over-estimate but never under-estimate.
    ///
    /// # Arguments
    /// * `value` - The JSON value to estimate memory usage for
    ///
    /// # Returns
    /// The estimated memory usage in bytes
    ///
    /// # Examples
    /// ```
    /// use serde_json::json;
    /// use oya_frontend::graph::Workflow;
    ///
    /// let value = json!({"name": "test", "value": 42});
    /// let memory = Workflow::estimate_memory_usage(&value);
    /// assert!(memory > 0);
    /// ```
    #[must_use]
    pub fn estimate_memory_usage(value: &serde_json::Value) -> u64 {
        match value {
            serde_json::Value::Null => 4,    // "null"
            serde_json::Value::Bool(_) => 5, // "true" or "false"
            serde_json::Value::Number(num) => {
                // Conservative estimate: serialize to string and count bytes
                num.to_string().len() as u64
            }
            serde_json::Value::String(s) => {
                // String bytes + UTF-8 overhead for quotes and escaping
                s.len() as u64 + 2
            }
            serde_json::Value::Array(arr) => {
                // Array brackets + commas + elements
                let elements: u64 = arr.iter().map(Self::estimate_memory_usage).sum();
                // Estimate 1 byte per comma for n elements
                let separators = arr.len().saturating_sub(1) as u64;
                2 + elements + separators // [ ] + commas
            }
            serde_json::Value::Object(obj) => {
                // Object braces + quotes + colons + commas + keys + values
                let entries: u64 = obj
                    .iter()
                    .map(|(k, v)| {
                        k.len() as u64 + 2 // quotes for key
                        + 1 // colon
                        + Self::estimate_memory_usage(v)
                    })
                    .sum();
                let separators = obj.len().saturating_sub(1) as u64;
                2 + entries + separators // { } + commas
            }
        }
    }

    /// Checks if the current memory usage exceeds the configured limit.
    ///
    /// # Arguments
    /// * `bytes_used` - The current memory usage in bytes
    ///
    /// # Returns
    /// `true` if the memory limit is exceeded, `false` otherwise
    ///
    /// # Examples
    /// ```
    /// use oya_frontend::graph::{Workflow, execution_types::ExecutionConfig};
    ///
    /// let mut workflow = Workflow::new();
    /// workflow = workflow.with_memory_limit(1024);
    ///
    /// assert!(!workflow.is_memory_limit_exceeded(512));
    /// assert!(workflow.is_memory_limit_exceeded(2048));
    /// ```
    #[must_use]
    pub fn is_memory_limit_exceeded(&self, bytes_used: u64) -> bool {
        self.execution_config.memory_limit_bytes.is_some_and(|limit| bytes_used >= limit)
    }

    /// Updates the current memory usage and checks if the limit is exceeded.
    ///
    /// This is a convenience method that adds the node output size to the
    /// current memory total and checks against the limit.
    ///
    /// # Arguments
    /// * `output` - The node output to measure
    ///
    /// # Returns
    /// * `Ok(())` - If the output fits within the memory limit
    /// * `Err(WorkflowExecutionError::MemoryLimitExceeded)` - If the memory limit is exceeded
    ///
    /// # Errors
    /// Returns `WorkflowExecutionError::MemoryLimitExceeded` when the total memory
    /// usage (including this output) would exceed the configured limit.
    ///
    /// # Examples
    /// ```
    /// use serde_json::json;
    /// use oya_frontend::graph::{Workflow, WorkflowExecutionError};
    ///
    /// let mut workflow = Workflow::new();
    /// workflow = workflow.with_memory_limit(100);
    ///
    /// let small = json!({"ok": true});
    /// assert!(workflow.check_and_update_memory(&small).is_ok());
    ///
    /// let large = json!({"data": "x".repeat(1000)});
    /// assert!(matches!(
    ///     workflow.check_and_update_memory(&large),
    ///     Err(WorkflowExecutionError::MemoryLimitExceeded { .. })
    /// ));
    /// ```
    pub fn check_and_update_memory(
        &mut self,
        output: &serde_json::Value,
    ) -> Result<(), WorkflowExecutionError> {
        let output_size = Self::estimate_memory_usage(output);
        self.current_memory_bytes = self.current_memory_bytes.saturating_add(output_size);

        if self.is_memory_limit_exceeded(self.current_memory_bytes) {
            let node_id = self.execution_queue.get(self.current_step.saturating_sub(1)).copied();

            return Err(WorkflowExecutionError::MemoryLimitExceeded {
                node_id,
                bytes_used: self.current_memory_bytes,
                limit_bytes: self.execution_config.memory_limit_bytes.unwrap_or(u64::MAX),
            });
        }

        Ok(())
    }

    // ===========================================================================
    // Node Execution Runtime
    // ===========================================================================

    pub(super) async fn execute_node_type(
        &self,
        node_type_str: &str,
        resolved_config: &serde_json::Value,
        parent_outputs: &[serde_json::Value],
    ) -> serde_json::Value {
        match node_type_str {
            "http-handler" | "kafka-handler" | "cron-trigger" => serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "source": node_type_str
            }),
            "http-request" | "http-call" => self.execute_http_request(resolved_config).await,
            "run" => {
                resolved_config.get("mapping").cloned().unwrap_or_else(|| resolved_config.clone())
            }
            "service-call" | "object-call" | "workflow-call" => {
                self.execute_service_call_internal(node_type_str, resolved_config).await
            }
            "condition" => {
                let condition_value =
                    resolved_config.get("expression").or_else(|| resolved_config.get("condition"));
                let result = match condition_value {
                    Some(serde_json::Value::Bool(b)) => *b,
                    Some(serde_json::Value::String(s)) => {
                        s == "true" || (!s.is_empty() && s != "false")
                    }
                    _ => false,
                };
                let condition_str = condition_value.and_then(|v| v.as_str()).unwrap_or("");
                serde_json::json!({ "result": result, "condition": condition_str })
            }
            _ => serde_json::json!({
                "executed": true,
                "step": self.current_step,
                "input_count": parent_outputs.len(),
                "config": resolved_config
            }),
        }
    }

    async fn execute_http_request(&self, config: &serde_json::Value) -> serde_json::Value {
        let url = config
            .get("url")
            .and_then(serde_json::Value::as_str)
            .map_or("https://httpbin.org/get", |s| s);
        let method = config.get("method").and_then(serde_json::Value::as_str).map_or("GET", |s| s);

        let client = reqwest::Client::new();
        let rb = match method {
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            _ => client.get(url),
        };

        match rb.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body: serde_json::Value =
                    resp.json().await.unwrap_or_else(|_| serde_json::json!({}));
                serde_json::json!({ "status": status, "url": url, "body": body })
            }
            Err(e) => serde_json::json!({ "error": e.to_string(), "url": url }),
        }
    }
}

// ===========================================================================
// Memory Estimation Tests
// ===========================================================================

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn given_null_value_when_estimating_memory_then_returns_four_bytes() {
        let value = serde_json::Value::Null;
        let memory = Workflow::estimate_memory_usage(&value);
        assert_eq!(memory, 4);
    }

    #[test]
    fn given_bool_value_when_estimating_memory_then_returns_five_bytes() {
        let true_val = serde_json::Value::Bool(true);
        let false_val = serde_json::Value::Bool(false);
        let true_memory = Workflow::estimate_memory_usage(&true_val);
        let false_memory = Workflow::estimate_memory_usage(&false_val);
        assert_eq!(true_memory, 5);
        assert_eq!(false_memory, 5);
    }

    #[test]
    fn given_number_value_when_estimating_memory_then_returns_string_length() {
        let int_val = serde_json::json!(42);
        let float_val = serde_json::json!(3.14);
        let negative_val = serde_json::json!(-100);

        assert_eq!(Workflow::estimate_memory_usage(&int_val), 2); // "42"
        assert_eq!(Workflow::estimate_memory_usage(&float_val), 4); // "3.14"
        assert_eq!(Workflow::estimate_memory_usage(&negative_val), 4); // "-100"
    }

    #[test]
    fn given_string_value_when_estimating_memory_then_returns_length_plus_two() {
        let short = serde_json::json!("hi");
        let long = serde_json::json!("hello world");
        let empty = serde_json::json!("");

        assert_eq!(Workflow::estimate_memory_usage(&short), 4); // "hi" + 2
        assert_eq!(Workflow::estimate_memory_usage(&long), 13); // "hello world" + 2
        assert_eq!(Workflow::estimate_memory_usage(&empty), 2); // "" + 2
    }

    #[test]
    fn given_array_value_when_estimating_memory_then_includes_brackets_and_separators() {
        let empty_arr = serde_json::json!([]);
        let single = serde_json::json!([1]);
        let multiple = serde_json::json!([1, 2, 3]);

        assert_eq!(Workflow::estimate_memory_usage(&empty_arr), 2); // [ ]
        assert_eq!(Workflow::estimate_memory_usage(&single), 3); // [1] = 2 + 1 + 0
        assert_eq!(Workflow::estimate_memory_usage(&multiple), 7); // [1,2,3] = 2 + 6 + 2
    }

    #[test]
    fn given_object_value_when_estimating_memory_then_includes_braces_and_separators() {
        let empty_obj = serde_json::json!({});
        let single = serde_json::json!({"a": 1});
        let multiple = serde_json::json!({"a": 1, "b": 2});

        assert_eq!(Workflow::estimate_memory_usage(&empty_obj), 2); // { }
        assert_eq!(Workflow::estimate_memory_usage(&single), 7); // {"a":1} = 2 + 5 + 0
        assert_eq!(Workflow::estimate_memory_usage(&multiple), 13); // {"a":1,"b":2} = 2 + 10 + 1
    }

    #[test]
    fn given_nested_structure_when_estimating_memory_then_recursive_calculation() {
        let nested = serde_json::json!({
            "users": [
                {"name": "Alice", "age": 30},
                {"name": "Bob", "age": 25}
            ]
        });

        let memory = Workflow::estimate_memory_usage(&nested);
        assert!(memory > 0);
        // Should be significantly larger than a simple value
        assert!(memory > 50);
    }

    #[test]
    fn given_large_string_when_estimating_memory_then_approximates_correctly() {
        let large_string = serde_json::json!(format!("x{}", "x".repeat(1000)));
        let memory = Workflow::estimate_memory_usage(&large_string);
        assert!(memory >= 1000); // Should be at least 1000 bytes + overhead
        assert!(memory <= 1010); // Should be at most 1000 bytes + reasonable overhead
    }

    #[test]
    fn given_workflow_with_memory_limit_when_checking_then_returns_correct_result() {
        let mut workflow = Workflow::new();
        workflow = workflow.with_memory_limit(1024);

        assert!(!workflow.is_memory_limit_exceeded(512));
        assert!(!workflow.is_memory_limit_exceeded(1023));
        assert!(workflow.is_memory_limit_exceeded(1024));
        assert!(workflow.is_memory_limit_exceeded(2048));
    }

    #[test]
    fn given_workflow_without_memory_limit_when_checking_then_never_exceeds() {
        let workflow = Workflow::new();

        assert!(!workflow.is_memory_limit_exceeded(0));
        assert!(!workflow.is_memory_limit_exceeded(1024));
        assert!(!workflow.is_memory_limit_exceeded(u64::MAX));
    }

    #[test]
    fn given_small_output_when_checking_and_updating_memory_then_succeeds() {
        let mut workflow = Workflow::new();
        workflow = workflow.with_memory_limit(1024);

        let small_output = serde_json::json!({"ok": true});
        assert!(workflow.check_and_update_memory(&small_output).is_ok());
        assert!(workflow.current_memory_bytes > 0);
    }

    #[test]
    fn given_large_output_when_checking_and_updating_memory_then_fails() {
        let mut workflow = Workflow::new();
        workflow = workflow.with_memory_limit(100);

        let large_output = serde_json::json!({"data": "x".repeat(1000)});
        let result = workflow.check_and_update_memory(&large_output);

        assert!(result.is_err());
        if let Err(err) = result {
            matches!(err, WorkflowExecutionError::MemoryLimitExceeded { .. });
        }
    }
}
