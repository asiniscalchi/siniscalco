use tracing_subscriber::{EnvFilter, fmt};

const DEFAULT_LOG_FILTER: &str = "backend=info,tower_http=info";

pub fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    fmt()
        .with_env_filter(env_filter())
        .with_target(false)
        .compact()
        .try_init()
}

pub fn default_log_filter() -> &'static str {
    DEFAULT_LOG_FILTER
}

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER))
}

#[cfg(test)]
mod tests {
    use super::default_log_filter;

    #[test]
    fn exposes_default_filter() {
        assert_eq!(default_log_filter(), "backend=info,tower_http=info");
    }
}
