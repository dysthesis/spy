use std::time::Duration;

use once_cell::sync::Lazy;
use ureq::Agent;

pub mod cli;
pub mod entry;
pub mod tag;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.10 Safari/605.1.1";

/// HTTP agent to use to fetch webpages
pub static AGENT: Lazy<Agent> = Lazy::new(|| {
    let config = Agent::config_builder()
        .user_agent(USER_AGENT)
        .timeout_global(Some(Duration::from_secs(10)))
        .build();
    let agent: Agent = config.into();
    agent
});
