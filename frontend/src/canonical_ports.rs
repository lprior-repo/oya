//! Canonical local endpoints for Oya's frontend-to-core integration.

pub const INGRESS_PORT: u16 = 909;
pub const ADMIN_PORT: u16 = 9070;
pub const SERVICE_PORT: u16 = 9180;

#[must_use]
pub fn endpoint_url(port: u16) -> String {
    format!("http://localhost:{port}")
}

#[must_use]
pub fn default_ingress_url() -> String {
    endpoint_url(INGRESS_PORT)
}

#[must_use]
pub fn default_admin_url() -> String {
    endpoint_url(ADMIN_PORT)
}

#[must_use]
pub fn default_service_url() -> String {
    endpoint_url(SERVICE_PORT)
}
