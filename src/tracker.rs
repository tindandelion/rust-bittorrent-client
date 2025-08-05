use crate::{
    bencoding::decode_dict,
    types::{PeerId, Sha1},
};
use std::{
    error::Error,
    net::{SocketAddr, ToSocketAddrs},
};
use url::{ParseError, Url};

pub struct AnnounceParams {
    pub info_hash: Sha1,
    pub peer_id: PeerId,
}

pub fn make_announce_request(
    tracker_url: &str,
    request_params: &AnnounceParams,
) -> Result<String, Box<dyn Error>> {
    let url = make_announce_url(tracker_url, request_params)?;
    let response = reqwest::blocking::get(url)?;
    Ok(response.text()?)
}

pub fn get_peer_list_from_response(
    tracker_response: &[u8],
) -> Result<Vec<SocketAddr>, Box<dyn Error>> {
    let decoded_response = decode_dict(tracker_response)?;

    let peers_list = decoded_response.get("peers").unwrap().as_list().unwrap();
    let x = peers_list
        .iter()
        .map(|peer| peer.as_dict().unwrap())
        .map(|peer| {
            let ip = peer
                .get("ip")
                .and_then(|v| v.as_byte_string())
                .map(|v| v.to_string())
                .unwrap();
            let port = peer
                .get("port")
                .and_then(|v| v.as_int())
                .map(|v| *v as u16)
                .unwrap();

            (ip, port)
        })
        .flat_map(|(ip, port)| {
            (ip.as_str(), port)
                .to_socket_addrs()
                .unwrap_or_else(|e| panic!("Can't get the peer address from {ip}:{port}: {e:?}"))
        })
        .collect();
    Ok(x)
}

fn make_announce_url(
    tracker_url: &str,
    announce_params: &AnnounceParams,
) -> Result<Url, ParseError> {
    let info_hash = unsafe { String::from_utf8_unchecked(announce_params.info_hash.to_vec()) };
    let peer_id = unsafe { String::from_utf8_unchecked(announce_params.peer_id.to_vec()) };
    Url::parse_with_params(
        tracker_url,
        &[("info_hash", &info_hash), ("peer_id", &peer_id)],
    )
}

#[cfg(test)]
mod tests {
    use crate::tracker::get_peer_list_from_response;

    use super::*;

    #[test]
    fn make_simplest_tracker_request_url() {
        let tracker_url = "http://localhost:8000/announce";
        let request_params = AnnounceParams {
            info_hash: Sha1::new([
                0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf1, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd,
                0xef, 0x12, 0x34, 0x56, 0x78, 0x9a,
            ]),
            peer_id: PeerId::default(),
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
            info_hash: Sha1::new([0x00; 20]),
            peer_id: PeerId::default(),
        };
        let result = make_announce_url(tracker_url, &request_params);
        assert_eq!(Err(ParseError::InvalidPort), result);
    }

    #[test]
    fn parse_tracker_response_and_get_peer_list() {
        let tracker_response = "d8:intervali900e5:peersld2:ip11:88.18.61.544:porti4666eed2:ip13:85.31.128.1114:porti52664eed2:ip13:95.58.175.2324:porti26163eed2:ip14:83.148.245.1864:porti51414eed2:ip14:15.204.231.2024:porti45548eed2:ip14:93.165.240.1044:porti56439eed2:ip14:193.148.16.2114:porti15981eed2:ip13:104.28.224.824:porti16570eed2:ip15:185.193.157.1874:porti25297eed2:ip14:37.120.185.2084:porti51413eed2:ip13:82.102.23.1394:porti39206eed2:ip14:92.101.157.2504:porti58130eed2:ip13:87.58.176.2384:porti62014eed2:ip13:87.58.176.2384:porti62004eed2:ip14:118.142.44.1464:porti6988eed2:ip10:95.33.0.764:porti22936eed2:ip13:73.196.29.1454:porti51413eed2:ip15:163.172.218.2154:porti31951eed2:ip13:63.210.25.1394:porti6886eed2:ip14:82.165.117.1884:porti1eed2:ip12:98.115.1.2084:porti50413eed2:ip15:109.226.251.1304:porti1230eed2:ip14:103.136.92.2524:porti14948eed2:ip14:193.32.127.2224:porti51765eed2:ip14:45.134.212.1014:porti46296eed2:ip13:82.65.230.1594:porti63812eed2:ip13:87.58.176.2384:porti62017eed2:ip13:189.46.193.814:porti9751eed2:ip14:217.174.206.674:porti51413eed2:ip14:183.107.103.254:porti51413eed2:ip13:81.201.16.2474:porti54694eed2:ip11:78.82.25.834:porti6887eed2:ip14:46.231.240.1874:porti50000eed2:ip12:134.3.183.424:porti58578eed2:ip13:73.81.101.1304:porti51414eed2:ip14:89.142.165.1314:porti51413eed2:ip13:82.24.182.2044:porti44346eed2:ip13:87.99.116.1484:porti51413eed2:ip13:87.58.176.2384:porti62015eed2:ip13:38.162.49.1954:porti6881eed2:ip13:82.64.112.1454:porti25561eed2:ip12:212.7.200.734:porti30151eed2:ip14:37.120.210.2114:porti9099eed2:ip12:37.112.5.2244:porti6881eed2:ip12:50.35.176.534:porti62904eed2:ip14:195.206.105.374:porti57402eed2:ip13:73.235.107.364:porti6881eed2:ip14:187.193.191.434:porti51765eed2:ip14:37.120.198.1724:porti12018eed2:ip14:185.21.216.1694:porti32774eeee";
        let peers = get_peer_list_from_response(tracker_response.as_bytes()).unwrap();

        assert_eq!(50, peers.len());
        assert_eq!("88.18.61.54", peers[0].ip().to_string());
        assert_eq!(4666, peers[0].port());
    }
}
