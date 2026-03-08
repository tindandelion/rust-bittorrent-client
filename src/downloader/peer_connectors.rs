mod channel_connector;
mod par_connector;

use std::time::Duration;

pub use channel_connector::ChannelConnector;
pub use par_connector::ParPeerConnector;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
