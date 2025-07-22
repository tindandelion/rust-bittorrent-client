use std::error::Error;

use bt_client::{
    AnnounceParams,
    downloader::{self, PeerChannel},
    get_peer_list_from_response, get_piece_hashes, make_announce_request, read_torrent_file,
    types::{Peer, PeerId, Sha1},
};

fn main() -> Result<(), Box<dyn Error>> {
    let peer_id = PeerId::default();
    let torrent_file_contents = read_torrent_file();
    let announce_url = torrent_file_contents
        .get("announce")
        .and_then(|v| v.as_byte_string())
        .map(|v| v.to_string())
        .expect("Unable to retrieve announce URL");
    let info = torrent_file_contents
        .get("info")
        .and_then(|v| v.as_dict())
        .expect("Unable to retrieve `info` key");
    let piece_hashes = info
        .get("pieces")
        .and_then(|v| v.as_byte_string())
        .map(get_piece_hashes)
        .expect("Unable to retrieve `pieces` key");
    let piece_length = info
        .get("piece length")
        .and_then(|v| v.as_int())
        .map(|v| *v as u32)
        .expect("Unable to retrieve `piece length` key");
    let file_length = info
        .get("length")
        .and_then(|v| v.as_int())
        .map(|v| *v as usize)
        .expect("Unable to retrieve `length` key");

    println!(
        "* Total pieces {}, piece length {}",
        piece_hashes.len(),
        piece_length
    );
    println!("\n* Your announce url is: {}", announce_url);

    let info_hash = *info.sha1();
    let announce_params = AnnounceParams { info_hash, peer_id };
    let response = make_announce_request(&announce_url, &announce_params)?;
    let peers = get_peer_list_from_response(response.as_bytes())?;
    println!("* Total {} peers", peers.len());

    println!("* Probing peers...");
    if let Some(mut downloader) = connect_to_first_available_peer(&peers, info_hash, peer_id) {
        println!("* Connected to peer: {:?}", downloader.peer_addr()?);

        let file_content = download_file(&mut downloader, piece_hashes, piece_length, file_length)?;
        println!(
            "* Received file, first 128 bytes: {}",
            hex::encode(&file_content[..128])
        );
    } else {
        println!("* No peer responded");
    }

    Ok(())
}

fn connect_to_first_available_peer(
    peers: &[Peer],
    info_hash: Sha1,
    peer_id: PeerId,
) -> Option<PeerChannel> {
    for peer in peers {
        print!("{}:{}\t-> ", peer.ip, peer.port);
        match probe_peer(peer, info_hash, peer_id) {
            Ok((result, downloader)) => {
                println!("OK({})", result);
                return Some(downloader);
            }
            Err(e) => println!("Err({})", e),
        }
    }
    None
}

fn probe_peer(
    peer: &Peer,
    info_hash: Sha1,
    peer_id: PeerId,
) -> Result<(String, PeerChannel), Box<dyn Error>> {
    let peer_addr = peer.to_socket_addr()?;
    let mut channel = PeerChannel::connect(&peer_addr)?;
    let handshake_result = channel.handshake(info_hash, peer_id)?;
    Ok((format!("{:?}", handshake_result), channel))
}

fn download_file(
    channel: &mut PeerChannel,
    piece_hashes: Vec<Sha1>,
    piece_length: u32,
    file_length: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let bitfield = channel.receive_bitfield()?;
    println!("* Received bitfield: {}", hex::encode(bitfield));

    println!("* Sending `interested` message");
    channel.send_interested()?;

    println!("* Receiving `unchoke` message");
    channel.receive_unchoke()?;

    println!("* Unchoked, requesting file");
    let file_content = downloader::download_file(channel, piece_hashes, piece_length, file_length)?;
    Ok(file_content)
}
