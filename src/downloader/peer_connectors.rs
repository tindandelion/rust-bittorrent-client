mod par_connector;
mod seq_connector;

use std::time::Duration;

pub use par_connector::ParPeerConnector;
pub use seq_connector::SeqPeerConnector;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
