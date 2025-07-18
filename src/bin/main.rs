use std::error::Error;

use bt_client::{
    AnnounceParams, FileDownloader, get_peer_list_from_response, make_announce_request,
    read_sample_file_part, read_torrent_file,
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

        let block_length = 128;
        let block = download_file_block(&mut downloader, block_length)?;
        let sample_data = read_sample_file_part(block_length as usize);
        if block == sample_data {
            println!("\n\n* RECEIVED BLOCK MATCHES SAMPLE DATA");
        } else {
            println!("\n\n* RECEIVED BLOCK DOES NOT MATCH SAMPLE DATA");
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

fn download_file_block(
    downloader: &mut FileDownloader,
    block_length: u32,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let bitfield = downloader.receive_bitfield()?;
    println!("* Received bitfield: {}", hex::encode(bitfield));

    println!("* Sending `interested` message");
    downloader.send_interested()?;

    println!("* Receiving `unchoke` message");
    downloader.receive_unchoke()?;

    println!("* Unchoked, requesting data block");
    downloader.request_block(0, 0, block_length)?;

    println!("* Receiving `piece` message");
    let (piece_index, offset, block) = downloader.receive_piece()?;
    println!(
        "* Received block of piece {} at offset {}: {} ",
        piece_index,
        offset,
        hex::encode(block.clone())
    );
    Ok(block)
}
