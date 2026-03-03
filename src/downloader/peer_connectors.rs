mod par_connector;

use std::time::Duration;

pub use par_connector::ParPeerConnector;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
