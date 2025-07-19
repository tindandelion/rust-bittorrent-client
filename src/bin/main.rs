use std::error::Error;

use bt_client::{
    AnnounceParams, FileDownloader, Piece, get_peer_list_from_response, make_announce_request,
    read_torrent_file,
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
    let pieces = info
        .get("pieces")
        .and_then(|v| v.as_byte_string())
        .expect("Unable to retrieve `pieces` key");
    let piece_length = info
        .get("piece length")
        .and_then(|v| v.as_int())
        .map(|v| *v as usize)
        .expect("Unable to retrieve `piece length` key");

    println!(
        "* Total pieces {}, piece length {}",
        pieces.len() / 20,
        piece_length
    );
    println!("\n* Your announce url is: {}", announce_url);

    let info_hash = *info.sha1();
    let announce_params = AnnounceParams { info_hash, peer_id };
    let response = make_announce_request(&announce_url, &announce_params)?;
    let peers = get_peer_list_from_response(&response.as_bytes())?;
    println!("* Total {} peers", peers.len());

    println!("* Probing peers...");
    if let Some(mut downloader) = connect_to_first_available_peer(&peers, info_hash, peer_id) {
        println!("* Connected to peer: {:?}", downloader.peer_addr()?);

        let expected_piece_hash = Sha1::from_bytes(&pieces.as_slice()[0..20]);
        let piece = download_piece(&mut downloader, 0, piece_length)?;
        println!("* Received piece: {}", hex::encode(&piece.bytes()[..128]));

        if piece.verify_hash(expected_piece_hash) {
            println!("* DOWNLOADED PIECE MATCHES EXPECTED HASH");
        } else {
            println!("* DOWNLOADED PIECE DOES NOT MATCH EXPECTED HASH");
        }
    } else {
        println!("* No peer responded");
    }

    Ok(())
}

fn connect_to_first_available_peer(
    peers: &[Peer],
    info_hash: Sha1,
    peer_id: PeerId,
) -> Option<FileDownloader> {
    for peer in peers {
        print!("{}:{}\t-> ", peer.ip, peer.port);
        match probe_peer(&peer, info_hash, peer_id) {
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
) -> Result<(String, FileDownloader), Box<dyn Error>> {
    let peer_addr = peer.to_socket_addr()?;
    let mut downloader = FileDownloader::connect(&peer_addr)?;
    let handshake_result = downloader.handshake(info_hash, peer_id)?;
    Ok((format!("{:?}", handshake_result), downloader))
}

fn download_piece(
    downloader: &mut FileDownloader,
    piece_index: u32,
    piece_length: usize,
) -> Result<Piece, Box<dyn Error>> {
    let bitfield = downloader.receive_bitfield()?;
    println!("* Received bitfield: {}", hex::encode(bitfield));

    println!("* Sending `interested` message");
    downloader.send_interested()?;

    println!("* Receiving `unchoke` message");
    downloader.receive_unchoke()?;

    println!("* Unchoked, requesting data block");
    let piece = downloader.download_piece(piece_index, piece_length)?;
    Ok(piece)
}
