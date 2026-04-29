//! Service call implementations.

use crate::graph::Workflow;
use serde_json::{json, Value};

enum ServiceCallRequest {
    Http { url: String, payload: Value },
    Noop,
}

impl Workflow {
    pub async fn execute_service_call_internal(&self, node_type: &str, config: &Value) -> Value {
        match build_service_call_request(&self.restate_ingress_url, node_type, config) {
            Ok(ServiceCallRequest::Http { url, payload }) => {
                post_restate_json(&url, &payload).await
            }
            Ok(ServiceCallRequest::Noop) => json!({ "executed": true }),
            Err(error) => error,
        }
    }
}

fn build_service_call_request(
    base: &str,
    node_type: &str,
    config: &Value,
) -> Result<ServiceCallRequest, Value> {
    match node_type {
        "service-call" => service_request(base, config),
        "object-call" => object_request(base, config),
        "workflow-call" => workflow_request(base, config),
        _ => Ok(ServiceCallRequest::Noop),
    }
}

fn service_request(base: &str, config: &Value) -> Result<ServiceCallRequest, Value> {
    let service = config_str(config, "service");
    let endpoint = config_str(config, "endpoint");
    if service.is_empty() || endpoint.is_empty() {
        return Err(json!({ "error": "service-call requires 'service' and 'endpoint' config" }));
    }

    Ok(http_request(format!("{base}/{service}/{endpoint}"), config))
}

fn object_request(base: &str, config: &Value) -> Result<ServiceCallRequest, Value> {
    let object = config_str(config, "object_name");
    let handler = config_str(config, "handler");
    if object.is_empty() || handler.is_empty() {
        return Err(json!({ "error": "object-call requires 'object_name' and 'handler' config" }));
    }

    let key = non_empty_or_default(config_str(config, "key"), "default");
    Ok(http_request(format!("{base}/{object}/{key}/{handler}"), config))
}

fn workflow_request(base: &str, config: &Value) -> Result<ServiceCallRequest, Value> {
    let workflow = config_str(config, "workflow_name");
    if workflow.is_empty() {
        return Err(json!({ "error": "workflow-call requires 'workflow_name' config" }));
    }

    let id = uuid::Uuid::new_v4();
    Ok(http_request(format!("{base}/{workflow}/{id}/run"), config))
}

fn http_request(url: String, config: &Value) -> ServiceCallRequest {
    ServiceCallRequest::Http { url, payload: request_payload(config) }
}

fn config_str<'a>(config: &'a Value, key: &str) -> &'a str {
    config.get(key).and_then(Value::as_str).map_or("", |value| value)
}

const fn non_empty_or_default<'a>(value: &'a str, default: &'a str) -> &'a str {
    if value.is_empty() {
        default
    } else {
        value
    }
}

fn request_payload(config: &Value) -> Value {
    config.get("payload").map_or_else(|| json!({}), Clone::clone)
}

async fn post_restate_json(url: &str, payload: &Value) -> Value {
    let client = reqwest::Client::new();
    match client.post(url).json(payload).send().await {
        Ok(resp) => parse_restate_response(resp).await,
        Err(err) => json!({ "error": err.to_string() }),
    }
}

async fn parse_restate_response(resp: reqwest::Response) -> Value {
    let status = resp.status().as_u16();
    match resp.json::<Value>().await {
        Ok(body) => restate_response_body(status, &body),
        Err(err) => json!({ "status": status, "error": err.to_string() }),
    }
}

fn restate_response_body(status: u16, body: &Value) -> Value {
    let inv_id = body.get("id").and_then(Value::as_str).map(str::to_string);
    json!({ "status": status, "restate_invocation_id": inv_id, "body": body })
}
