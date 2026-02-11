//! System health monitoring and dashboard data structures
//!
//! Provides immutable data structures for tracking system health status,
//! component health, resource usage, and throughput metrics.
//! All structures use functional patterns with zero mutation.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use rpds::Vector;
use thiserror::Error;

/// Errors that can occur in health monitoring
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum HealthError {
    #[error("Invalid health status: {0}")]
    InvalidStatus(String),

    #[error("Invalid metric value: {0}")]
    InvalidMetric(String),
}

/// System-wide health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemStatus {
    /// All systems operational
    Healthy,
    /// Some components degraded but functioning
    Degraded,
    /// Critical failures, system impaired
    Unhealthy,
}

impl SystemStatus {
    /// Get display symbol for status
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Healthy => "✓",
            Self::Degraded => "⚠",
            Self::Unhealthy => "✗",
        }
    }

    /// Get display color code for status
    #[must_use]
    pub const fn color(self) -> &'static str {
        match self {
            Self::Healthy => "\x1b[32m",   // Green
            Self::Degraded => "\x1b[33m",  // Yellow
            Self::Unhealthy => "\x1b[31m", // Red
        }
    }
}

/// Individual component health
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentHealth {
    /// Component name
    pub name: String,
    /// Component status
    pub status: SystemStatus,
    /// Optional status message
    pub message: Option<String>,
    /// Last update timestamp (unix seconds)
    pub timestamp: i64,
}

impl ComponentHealth {
    /// Create a new component health entry
    #[must_use]
    pub fn new(name: impl Into<String>, status: SystemStatus, timestamp: i64) -> Self {
        Self {
            name: name.into(),
            status,
            message: None,
            timestamp,
        }
    }

    /// Add a status message
    #[must_use]
    pub fn with_message(self, message: impl Into<String>) -> Self {
        Self {
            message: Some(message.into()),
            ..self
        }
    }
}

/// Resource usage metrics
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceUsage {
    /// CPU usage percentage (0.0 - 100.0)
    pub cpu_percent: f64,
    /// Memory usage in MB
    pub memory_mb: f64,
    /// Disk usage in GB
    pub disk_gb: f64,
    /// Total memory available in MB
    pub total_memory_mb: f64,
    /// Total disk available in GB
    pub total_disk_gb: f64,
}

impl ResourceUsage {
    /// Create new resource usage metrics
    #[must_use]
    pub const fn new(
        cpu_percent: f64,
        memory_mb: f64,
        disk_gb: f64,
        total_memory_mb: f64,
        total_disk_gb: f64,
    ) -> Self {
        Self {
            cpu_percent,
            memory_mb,
            disk_gb,
            total_memory_mb,
            total_disk_gb,
        }
    }

    /// Calculate memory usage percentage
    #[must_use]
    pub fn memory_percent(self) -> f64 {
        if self.total_memory_mb <= 0.0 {
            return 0.0;
        }
        (self.memory_mb / self.total_memory_mb * 100.0).clamp(0.0, 100.0)
    }

    /// Calculate disk usage percentage
    #[must_use]
    pub fn disk_percent(self) -> f64 {
        if self.total_disk_gb <= 0.0 {
            return 0.0;
        }
        (self.disk_gb / self.total_disk_gb * 100.0).clamp(0.0, 100.0)
    }
}

/// Throughput metrics
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThroughputMetrics {
    /// Requests per second
    pub requests_per_sec: f64,
    /// Bytes per second
    pub bytes_per_sec: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// P99 latency in milliseconds
    pub p99_latency_ms: f64,
}

impl ThroughputMetrics {
    /// Create new throughput metrics
    #[must_use]
    pub const fn new(
        requests_per_sec: f64,
        bytes_per_sec: f64,
        avg_latency_ms: f64,
        p99_latency_ms: f64,
    ) -> Self {
        Self {
            requests_per_sec,
            bytes_per_sec,
            avg_latency_ms,
            p99_latency_ms,
        }
    }

    /// Format bytes per second as human readable string
    #[must_use]
    pub fn bytes_per_sec_formatted(self) -> String {
        if self.bytes_per_sec >= 1_000_000_000.0 {
            format!("{:.2} GB/s", self.bytes_per_sec / 1_000_000_000.0)
        } else if self.bytes_per_sec >= 1_000_000.0 {
            format!("{:.2} MB/s", self.bytes_per_sec / 1_000_000.0)
        } else if self.bytes_per_sec >= 1_000.0 {
            format!("{:.2} KB/s", self.bytes_per_sec / 1_000.0)
        } else {
            format!("{:.0} B/s", self.bytes_per_sec)
        }
    }
}

/// System health snapshot containing all health data
#[derive(Debug, Clone, PartialEq)]
pub struct SystemHealthSnapshot {
    /// Overall system status
    pub status: SystemStatus,
    /// Individual component health
    pub components: Vector<ComponentHealth>,
    /// Resource usage metrics
    pub resources: ResourceUsage,
    /// Throughput metrics
    pub throughput: ThroughputMetrics,
    /// Snapshot timestamp (unix seconds)
    pub timestamp: i64,
}

impl SystemHealthSnapshot {
    /// Create a new system health snapshot
    #[must_use]
    pub fn new(
        status: SystemStatus,
        components: Vector<ComponentHealth>,
        resources: ResourceUsage,
        throughput: ThroughputMetrics,
        timestamp: i64,
    ) -> Self {
        Self {
            status,
            components,
            resources,
            throughput,
            timestamp,
        }
    }

    /// Count components by status using functional patterns
    #[must_use]
    pub fn count_by_status(&self, status: SystemStatus) -> usize {
        self.components
            .iter()
            .filter(|c| c.status == status)
            .count()
    }

    /// Get the most critical component status
    #[must_use]
    pub fn derive_status_from_components(&self) -> SystemStatus {
        let has_unhealthy = self
            .components
            .iter()
            .any(|c| c.status == SystemStatus::Unhealthy);
        let has_degraded = self
            .components
            .iter()
            .any(|c| c.status == SystemStatus::Degraded);

        if has_unhealthy {
            SystemStatus::Unhealthy
        } else if has_degraded {
            SystemStatus::Degraded
        } else {
            SystemStatus::Healthy
        }
    }

    /// Format for display in Zellij dashboard
    #[must_use]
    pub fn format_dashboard(&self, width: usize) -> String {
        let _inner_width = width.saturating_sub(2);
        let mut output = String::new();

        // Header
        output.push_str(self.status.color());
        output.push_str("System Health");
        output.push_str("\x1b[0m\n");

        output.push_str("Status: ");
        output.push_str(self.status.symbol());
        output.push(' ');
        output.push_str(&format!("{:?}", self.status));
        output.push_str("\x1b[0m\n\n");

        // Components section
        output.push_str("Components:\n");
        let component_lines = self.components.iter().fold(String::new(), |mut acc, c| {
            let msg = c.message.as_deref().unwrap_or("");
            acc.push_str("  ");
            acc.push_str(c.status.symbol());
            acc.push(' ');
            acc.push_str(&c.name);
            acc.push(' ');
            acc.push_str(msg);
            acc.push('\n');
            acc
        });
        output.push_str(&component_lines);
        output.push('\n');

        // Resources section
        output.push_str("Resources:\n  CPU: ");
        output.push_str(&format!("{:.1}", self.resources.cpu_percent));
        output.push_str("%\n  Memory: ");
        output.push_str(&format!("{:.1}", self.resources.memory_mb));
        output.push_str(" MB / ");
        output.push_str(&format!("{:.1}", self.resources.total_memory_mb));
        output.push_str(" MB (");
        output.push_str(&format!("{:.1}", self.resources.memory_percent()));
        output.push_str("%)\n  Disk: ");
        output.push_str(&format!("{:.1}", self.resources.disk_gb));
        output.push_str(" GB / ");
        output.push_str(&format!("{:.1}", self.resources.total_disk_gb));
        output.push_str(" GB (");
        output.push_str(&format!("{:.1}", self.resources.disk_percent()));
        output.push_str("%)\n\n");

        // Throughput section
        output.push_str("Throughput:\n  Requests/sec: ");
        output.push_str(&format!("{:.1}", self.throughput.requests_per_sec));
        output.push_str("\n  Bytes/sec: ");
        output.push_str(&self.throughput.bytes_per_sec_formatted());
        output.push_str("\n  Latency: ");
        output.push_str(&format!("{:.2}", self.throughput.avg_latency_ms));
        output.push_str("ms (avg), ");
        output.push_str(&format!("{:.2}", self.throughput.p99_latency_ms));
        output.push_str("ms (p99)\n");

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_status_symbols() {
        assert_eq!(SystemStatus::Healthy.symbol(), "✓");
        assert_eq!(SystemStatus::Degraded.symbol(), "⚠");
        assert_eq!(SystemStatus::Unhealthy.symbol(), "✗");
    }

    #[test]
    fn test_system_status_colors() {
        assert!(SystemStatus::Healthy.color().contains("32m")); // Green
        assert!(SystemStatus::Degraded.color().contains("33m")); // Yellow
        assert!(SystemStatus::Unhealthy.color().contains("31m")); // Red
    }

    #[test]
    fn test_component_health_creation() {
        let component = ComponentHealth::new("test-service", SystemStatus::Healthy, 1234567890);
        assert_eq!(component.name, "test-service");
        assert_eq!(component.status, SystemStatus::Healthy);
        assert_eq!(component.timestamp, 1234567890);
        assert!(component.message.is_none());
    }

    #[test]
    fn test_component_health_with_message() {
        let component = ComponentHealth::new("test-service", SystemStatus::Healthy, 1234567890)
            .with_message("All systems operational");
        assert_eq!(
            component.message,
            Some("All systems operational".to_string())
        );
    }

    #[test]
    fn test_resource_usage_creation() {
        let resources = ResourceUsage::new(45.5, 2048.0, 50.0, 8192.0, 500.0);
        assert_eq!(resources.cpu_percent, 45.5);
        assert_eq!(resources.memory_mb, 2048.0);
        assert_eq!(resources.disk_gb, 50.0);
        assert_eq!(resources.total_memory_mb, 8192.0);
        assert_eq!(resources.total_disk_gb, 500.0);
    }

    #[test]
    fn test_resource_usage_percentages() {
        let resources = ResourceUsage::new(45.5, 2048.0, 50.0, 8192.0, 500.0);
        assert!((resources.memory_percent() - 25.0).abs() < 0.1);
        assert!((resources.disk_percent() - 10.0).abs() < 0.1);
    }

    #[test]
    fn test_resource_usage_zero_totals() {
        let resources = ResourceUsage::new(45.5, 2048.0, 50.0, 0.0, 0.0);
        assert_eq!(resources.memory_percent(), 0.0);
        assert_eq!(resources.disk_percent(), 0.0);
    }

    #[test]
    fn test_throughput_metrics_creation() {
        let throughput = ThroughputMetrics::new(100.5, 1024000.0, 15.5, 45.0);
        assert_eq!(throughput.requests_per_sec, 100.5);
        assert_eq!(throughput.bytes_per_sec, 1024000.0);
        assert_eq!(throughput.avg_latency_ms, 15.5);
        assert_eq!(throughput.p99_latency_ms, 45.0);
    }

    #[test]
    fn test_throughput_bytes_formatting() {
        let tb = ThroughputMetrics::new(0.0, 1024000.0, 0.0, 0.0);
        assert!(tb.bytes_per_sec_formatted().contains("MB/s"));

        let gb = ThroughputMetrics::new(0.0, 2_000_000_000.0, 0.0, 0.0);
        assert!(gb.bytes_per_sec_formatted().contains("GB/s"));

        let kb = ThroughputMetrics::new(0.0, 5000.0, 0.0, 0.0);
        assert!(kb.bytes_per_sec_formatted().contains("KB/s"));

        let b = ThroughputMetrics::new(0.0, 500.0, 0.0, 0.0);
        assert!(b.bytes_per_sec_formatted().contains("B/s"));
    }

    #[test]
    fn test_system_health_snapshot_creation() {
        let components = Vector::from_iter(vec![ComponentHealth::new(
            "api",
            SystemStatus::Healthy,
            1234567890,
        )]);
        let resources = ResourceUsage::new(45.5, 2048.0, 50.0, 8192.0, 500.0);
        let throughput = ThroughputMetrics::new(100.5, 1024000.0, 15.5, 45.0);

        let snapshot = SystemHealthSnapshot::new(
            SystemStatus::Healthy,
            components,
            resources,
            throughput,
            1234567890,
        );

        assert_eq!(snapshot.status, SystemStatus::Healthy);
        assert_eq!(snapshot.timestamp, 1234567890);
    }

    #[test]
    fn test_count_by_status() {
        let components = Vector::from_iter(vec![
            ComponentHealth::new("api", SystemStatus::Healthy, 1234567890),
            ComponentHealth::new("db", SystemStatus::Healthy, 1234567890),
            ComponentHealth::new("cache", SystemStatus::Degraded, 1234567890),
            ComponentHealth::new("worker", SystemStatus::Unhealthy, 1234567890),
        ]);
        let resources = ResourceUsage::new(0.0, 0.0, 0.0, 1.0, 1.0);
        let throughput = ThroughputMetrics::new(0.0, 0.0, 0.0, 0.0);

        let snapshot = SystemHealthSnapshot::new(
            SystemStatus::Degraded,
            components,
            resources,
            throughput,
            1234567890,
        );

        assert_eq!(snapshot.count_by_status(SystemStatus::Healthy), 2);
        assert_eq!(snapshot.count_by_status(SystemStatus::Degraded), 1);
        assert_eq!(snapshot.count_by_status(SystemStatus::Unhealthy), 1);
    }

    #[test]
    fn test_derive_status_from_components() {
        let healthy_components = Vector::from_iter(vec![
            ComponentHealth::new("api", SystemStatus::Healthy, 1234567890),
            ComponentHealth::new("db", SystemStatus::Healthy, 1234567890),
        ]);
        let resources = ResourceUsage::new(0.0, 0.0, 0.0, 1.0, 1.0);
        let throughput = ThroughputMetrics::new(0.0, 0.0, 0.0, 0.0);

        let snapshot = SystemHealthSnapshot::new(
            SystemStatus::Healthy,
            healthy_components.clone(),
            resources,
            throughput,
            1234567890,
        );
        assert_eq!(
            snapshot.derive_status_from_components(),
            SystemStatus::Healthy
        );

        let degraded_components = Vector::from_iter(vec![
            ComponentHealth::new("api", SystemStatus::Healthy, 1234567890),
            ComponentHealth::new("cache", SystemStatus::Degraded, 1234567890),
        ]);
        let snapshot2 = SystemHealthSnapshot::new(
            SystemStatus::Degraded,
            degraded_components,
            resources,
            throughput,
            1234567890,
        );
        assert_eq!(
            snapshot2.derive_status_from_components(),
            SystemStatus::Degraded
        );

        let unhealthy_components = Vector::from_iter(vec![
            ComponentHealth::new("api", SystemStatus::Healthy, 1234567890),
            ComponentHealth::new("worker", SystemStatus::Unhealthy, 1234567890),
        ]);
        let snapshot3 = SystemHealthSnapshot::new(
            SystemStatus::Unhealthy,
            unhealthy_components,
            resources,
            throughput,
            1234567890,
        );
        assert_eq!(
            snapshot3.derive_status_from_components(),
            SystemStatus::Unhealthy
        );
    }

    #[test]
    fn test_format_dashboard() {
        let components = Vector::from_iter(vec![ComponentHealth::new(
            "api",
            SystemStatus::Healthy,
            1234567890,
        )
        .with_message("Operational")]);
        let resources = ResourceUsage::new(45.5, 2048.0, 50.0, 8192.0, 500.0);
        let throughput = ThroughputMetrics::new(100.5, 1024000.0, 15.5, 45.0);

        let snapshot = SystemHealthSnapshot::new(
            SystemStatus::Healthy,
            components,
            resources,
            throughput,
            1234567890,
        );

        let output = snapshot.format_dashboard(80);
        assert!(output.contains("System Health"));
        assert!(output.contains("Healthy"));
        assert!(output.contains("api"));
        assert!(output.contains("CPU:"));
        assert!(output.contains("Memory:"));
        assert!(output.contains("Requests/sec:"));
    }
}
