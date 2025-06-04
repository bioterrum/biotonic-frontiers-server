//! Prometheus metrics & middleware helper.

use actix_web_prom::{PrometheusMetrics, PrometheusMetricsBuilder};
use once_cell::sync::Lazy;

/// Global Prometheus handle reused in tests.
pub static METRICS: Lazy<PrometheusMetrics> = Lazy::new(|| {
    PrometheusMetricsBuilder::new("api")
        .endpoint("/metrics") // exposed URL
        .build()
        .expect("metrics builder")
});
