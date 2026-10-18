use std::sync::OnceLock;
use std::time::Duration;

// If no data is read for this period of time during a file download, the download will be failed.
const REQUEST_TIMEOUT_READ_SECS: u64 = 20;
// If no data is written for this period of time during a file download, the download will be failed
const REQUEST_TIMEOUT_WRITE_SECS: u64 = 20;

/// The ureq agent used by MBF for downloads
static AGENT: OnceLock<ureq::Agent> = OnceLock::new();

pub fn get_agent() -> &'static ureq::Agent {
    AGENT.get_or_init(|| {
        let config = ureq::Agent::config_builder()
            .timeout_recv_body(Some(Duration::from_secs(REQUEST_TIMEOUT_READ_SECS)))
            .timeout_send_body(Some(Duration::from_secs(REQUEST_TIMEOUT_WRITE_SECS)))
            .https_only(true)
            .proxy(ureq::Proxy::try_from_env())
            .user_agent(format!("mbf-agent/{}", env!("CARGO_PKG_VERSION")))
            .build();

        ureq::Agent::new_with_config(config)
    })
}
