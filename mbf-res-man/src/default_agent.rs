use std::time::Duration;
use std::sync::OnceLock;

// If no data is read for this period of time during a file download, the download will be failed.
const REQUEST_TIMEOUT_READ_SECS: u64 = 20;
// If no data is written for this period of time during a file download, the download will be failed
const REQUEST_TIMEOUT_WRITE_SECS: u64 = 20;

/// The ureq agent used by MBF for downloads
static AGENT: OnceLock<ureq::Agent> = OnceLock::new();

pub fn get_agent() -> &'static ureq::Agent {
    AGENT.get_or_init(|| {
        ureq::AgentBuilder::new()
            .timeout_read(Duration::from_secs(REQUEST_TIMEOUT_READ_SECS))
            .timeout_write(Duration::from_secs(REQUEST_TIMEOUT_WRITE_SECS))
            .https_only(true)
            .try_proxy_from_env(true)
            .user_agent(format!("mbf-agent/{}", env!("CARGO_PKG_VERSION")).as_str())
            .build()
    })
}