use super::super::expressions::ExpressionContext;
use super::super::Workflow;

// ===========================================================================
// Expression Resolution
// ===========================================================================

impl Workflow {
    /// Resolves `{{expression}}` placeholders in a JSON config value.
    #[must_use]
    pub fn resolve_expressions(&self, config: &serde_json::Value) -> serde_json::Value {
        self.resolve_expressions_with_depth(config, 0)
    }

    /// Resolves expressions with a depth limit to prevent stack overflow.
    /// Max depth is 100 to prevent excessive recursion.
    pub(super) fn resolve_expressions_with_depth(
        &self,
        config: &serde_json::Value,
        depth: usize,
    ) -> serde_json::Value {
        // MAJOR: Enforce depth limit to prevent stack overflow
        const MAX_DEPTH: usize = 100;
        if depth > MAX_DEPTH {
            // Return config unchanged if depth exceeded
            return config.clone();
        }

        let ctx = ExpressionContext::new(&self.nodes);
        match config {
            serde_json::Value::String(s) => {
                if s.starts_with("{{") && s.ends_with("}}") {
                    let inner = s[2..s.len() - 2].trim();
                    return ctx.resolve(inner);
                }
                config.clone()
            }
            serde_json::Value::Object(map) => {
                let new_map = map
                    .iter()
                    .map(|(k, v)| (k.clone(), self.resolve_expressions_with_depth(v, depth + 1)))
                    .collect();
                serde_json::Value::Object(new_map)
            }
            serde_json::Value::Array(arr) => serde_json::Value::Array(
                arr.iter()
                    .map(|v| self.resolve_expressions_with_depth(v, depth + 1))
                    .collect(),
            ),
            serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) => {
                config.clone()
            }
        }
    }
}
