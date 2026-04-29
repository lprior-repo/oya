#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![forbid(unsafe_code)]

use crate::graph::Node;
use serde_json::Value;

pub struct ExpressionContext<'a> {
    pub nodes: &'a [Node],
}

impl<'a> ExpressionContext<'a> {
    #[must_use]
    pub const fn new(nodes: &'a [Node]) -> Self {
        Self { nodes }
    }

    #[must_use]
    pub fn resolve(&self, expr: &str) -> Value {
        let trimmed = expr.trim();

        if let Some(value) = self.resolve_node_path(trimmed) {
            return value;
        }
        if let Some(value) = self.resolve_binary(trimmed) {
            return value;
        }
        if let Some(value) = self.resolve_string_method(trimmed) {
            return value;
        }
        if let Some(value) = resolve_literal(trimmed) {
            return value;
        }

        Value::String(trimmed.to_string())
    }

    fn resolve_node_path(&self, trimmed: &str) -> Option<Value> {
        let node_part = trimmed.strip_prefix("$node[\"")?;
        let (node_name, path_part) = node_part.split_once("\"]")?;
        let path = path_part.strip_prefix(".json.").map_or(path_part, |stripped| stripped);
        let pointer = format!("/{}", path.replace('.', "/"));
        let resolved = self
            .nodes
            .iter()
            .find(|node| node.name == node_name)
            .and_then(|node| node.last_output.as_ref())
            .and_then(|out| out.pointer(&pointer));

        Some(resolved.map_or(Value::Null, Clone::clone))
    }

    fn resolve_binary(&self, trimmed: &str) -> Option<Value> {
        if let Some((left, right)) = trimmed.split_once(" + ") {
            return Some(self.eval_binary_op(left, right, |a, b| Value::from(a + b)));
        }
        trimmed
            .split_once(" - ")
            .map(|(left, right)| self.eval_binary_op(left, right, |a, b| Value::from(a - b)))
    }

    fn resolve_string_method(&self, trimmed: &str) -> Option<Value> {
        if let Some(base) = trimmed.strip_suffix(".to_uppercase()") {
            return self.resolve_uppercase(base);
        }
        trimmed.strip_suffix(".len()").and_then(|base| self.resolve_len(base))
    }

    fn resolve_uppercase(&self, base: &str) -> Option<Value> {
        self.resolve(base).as_str().map(|value| Value::String(value.to_uppercase()))
    }

    fn resolve_len(&self, base: &str) -> Option<Value> {
        let value = self.resolve(base);
        if let Some(text) = value.as_str() {
            return Some(Value::from(text.len()));
        }
        value.as_array().map(|array| Value::from(array.len()))
    }

    fn eval_binary_op<F>(&self, left: &str, right: &str, op: F) -> Value
    where
        F: Fn(f64, f64) -> Value,
    {
        let lv = self.resolve(left);
        let rv = self.resolve(right);
        if let (Some(l), Some(r)) = (lv.as_f64(), rv.as_f64()) {
            return op(l, r);
        }
        Value::Null
    }
}

fn resolve_literal(trimmed: &str) -> Option<Value> {
    if let Ok(n) = trimmed.parse::<f64>() {
        return Some(Value::from(n));
    }
    if let Some(value) = resolve_quoted_literal(trimmed) {
        return Some(value);
    }
    match trimmed {
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ => None,
    }
}

fn resolve_quoted_literal(trimmed: &str) -> Option<Value> {
    let quote = trimmed.chars().next()?;
    if !matches!(quote, '"' | '\'') || !trimmed.ends_with(quote) {
        return None;
    }
    trimmed
        .strip_prefix(quote)
        .and_then(|value| value.strip_suffix(quote))
        .map(|inner| Value::String(inner.to_string()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]
mod tests {
    use super::ExpressionContext;
    use crate::graph::Node;
    use serde_json::json;

    fn node_with_output(name: &str, output: serde_json::Value) -> Node {
        let mut node = Node::default();
        node.name = name.to_string();
        node.last_output = Some(output);
        node
    }

    #[test]
    fn given_single_quote_token_when_resolving_then_it_does_not_panic() {
        let ctx = ExpressionContext::new(&[]);

        let value = ctx.resolve("'");

        assert_eq!(value, serde_json::Value::String("'".to_string()));
    }

    #[test]
    fn given_double_quote_token_when_resolving_then_it_does_not_panic() {
        let ctx = ExpressionContext::new(&[]);

        let value = ctx.resolve("\"");

        assert_eq!(value, serde_json::Value::String("\"".to_string()));
    }

    #[test]
    fn given_wrapped_literal_when_resolving_then_quotes_are_trimmed() {
        let ctx = ExpressionContext::new(&[]);

        let value = ctx.resolve("'hello'");

        assert_eq!(value, serde_json::Value::String("hello".to_string()));
    }

    #[test]
    fn given_node_json_path_expression_when_resolving_then_returns_pointer_value() {
        let node = node_with_output("Fetcher", json!({"user": {"email": "a@b.dev"}}));
        let nodes = [node];
        let ctx = ExpressionContext::new(&nodes);

        let value = ctx.resolve("$node[\"Fetcher\"].json.user.email");

        assert_eq!(value, serde_json::Value::String("a@b.dev".to_string()));
    }

    #[test]
    fn given_numeric_binary_expression_when_resolving_then_returns_computed_number() {
        let ctx = ExpressionContext::new(&[]);

        assert_eq!(ctx.resolve("3 + 4"), serde_json::Value::from(7.0));
        assert_eq!(ctx.resolve("9 - 2"), serde_json::Value::from(7.0));
    }

    #[test]
    fn given_len_calls_when_resolving_then_returns_string_or_array_length() {
        let node = node_with_output("Fetcher", json!({"names": ["a", "b", "c"]}));
        let nodes = [node];
        let ctx = ExpressionContext::new(&nodes);

        assert_eq!(ctx.resolve("'hello'.len()"), serde_json::Value::from(5));
        assert_eq!(ctx.resolve("$node[\"Fetcher\"].json.names.len()"), serde_json::Value::Null);
    }

    #[test]
    fn given_uppercase_call_when_resolving_then_string_is_transformed() {
        let ctx = ExpressionContext::new(&[]);

        let value = ctx.resolve("'hello'.to_uppercase()");

        assert_eq!(value, serde_json::Value::String("HELLO".to_string()));
    }

    #[test]
    fn given_unknown_token_when_resolving_then_original_trimmed_string_is_returned() {
        let ctx = ExpressionContext::new(&[]);

        let value = ctx.resolve("  no_such_token  ");

        assert_eq!(value, serde_json::Value::String("no_such_token".to_string()));
    }
}
