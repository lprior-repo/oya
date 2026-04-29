use oya_frontend::canonical_ports::{
    default_admin_url, default_ingress_url, default_service_url, ADMIN_PORT, INGRESS_PORT,
    SERVICE_PORT,
};
use oya_frontend::graph::Workflow;
use oya_frontend::ui::app_bootstrap::default_workflow;

#[test]
fn frontend_ports_default_to_oya_canonical_endpoints() {
    assert_eq!(INGRESS_PORT, 909);
    assert_eq!(ADMIN_PORT, 9070);
    assert_eq!(SERVICE_PORT, 9180);
    assert_eq!(default_ingress_url(), "http://localhost:909");
    assert_eq!(default_admin_url(), "http://localhost:9070");
    assert_eq!(default_service_url(), "http://localhost:9180");
}

#[test]
fn frontend_ports_workflow_defaults_use_canonical_ingress_port() {
    assert_eq!(Workflow::new().restate_ingress_url, default_ingress_url());
    assert_eq!(default_workflow().restate_ingress_url, default_ingress_url());
}
