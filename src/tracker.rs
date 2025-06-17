use std::error::Error;

use url::{ParseError, Url};

pub struct AnnounceParams {
    pub info_hash: Vec<u8>,
    pub peer_id: Vec<u8>,
}

pub fn make_announce_request(
    tracker_url: &str,
    request_params: &AnnounceParams,
) -> Result<String, Box<dyn Error>> {
    let url = make_announce_url(tracker_url, request_params)?;
    let response = reqwest::blocking::get(url)?;
    Ok(response.text()?)
}

fn make_announce_url(
    tracker_url: &str,
    announce_params: &AnnounceParams,
) -> Result<Url, ParseError> {
    let info_hash = unsafe { String::from_utf8_unchecked(announce_params.info_hash.clone()) };
    let peer_id = unsafe { String::from_utf8_unchecked(announce_params.peer_id.clone()) };
    Url::parse_with_params(
        tracker_url,
        &[("info_hash", &info_hash), ("peer_id", &peer_id)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_simplest_tracker_request_url() {
        let tracker_url = "http://localhost:8000/announce";
        let request_params = AnnounceParams {
            info_hash: vec![
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf1, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd,
                0xef, 0x12, 0x34, 0x56, 0x78, 0x9a,
            ],
            peer_id: vec![0x00; 20],
        };

        let url = make_announce_url(tracker_url, &request_params).unwrap();

        let expected_params = [
            "info_hash=%124Vx%9A%BC%DE%F1%23Eg%89%AB%CD%EF%124Vx%9A",
            "peer_id=%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00%00",
        ]
        .join("&");
        let full_expected_url = tracker_url.to_owned() + "?" + &expected_params;
        assert_eq!(full_expected_url, url.to_string())
    }

    #[test]
    fn invalid_tracker_url_returns_error() {
        let tracker_url = "http://localhost:blah/announce";
        let request_params = AnnounceParams {
            info_hash: vec![0x00; 20],
            peer_id: vec![0x00; 20],
        };
        let result = make_announce_url(tracker_url, &request_params);
        assert_eq!(Err(ParseError::InvalidPort), result);
    }
}
