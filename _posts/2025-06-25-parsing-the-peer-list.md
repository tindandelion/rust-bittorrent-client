---
layout: post
title:  "Parsing the peer list from tracker"
date: 2025-06-25
---

[Last time][prev-post] we managed to fetch the list of peers from the torrent tracker for our [sample torrent file][torrent-file-0.0.4]. I left off by simply dumping the response from the torrent tracker onto the screen, and now I would like to pick up on that and actually parse the tracker's response, so that we can get our hands on peers' IP addresses and ports. That's going to be our next step towards making the connection to peers. 

# Tracker response structure 

Looking at the [description of tracker response][wiki-tracker-response], we can see that it's a bencoded dictionary with a few fields. Now for us the most interesting one is `peers` field, that contains a list of dictionaries, where each entry includes the information about an individual peer: 

* `ip`: peer's IP address (string); 
* `port`: peer's port number (integer). 

Indeed, we can see these values in the [raw tracker response][raw-tracker-response]. The specification also mentions the field `peer id`, but skimming through the response string, I don't see any traces of that field inside. I assume that field is optional. In any case, `ip` and `port` are the most important for us now. 

# A need for more powerful decoder 

Now, that's a fairly complex structure: a dictionary that contains a field that's a list of dictionaries. Recall that in our previous work we implemented _some_ decoding of bencoded values, but the functionality in this area is still quite basic. In particular, we have no way of parsing nested complex structures yet. 

Also, I've been adding accessor methods to the `Dict` struct in a somewhat haphazard manner, guided by what data I needed at the moment, which resulted in `Dict` having a few bespoke methods, such as `get_string()` and `get_dict_sha1()`. If I continue in that manner, I risk polluting the `Dict` interface with more methods of that kind. That's no bueno. 

It looks to me that we've come to the point when we need to pay more attention to parsing the bencoded data: 

* Our `Decoder` must be able to handle complex nested data structures, such as dictionaries containing lists of dictionaries; 
* We need a more coherent data model to represent the decoded data, so that we can work with it through a relatively narrow interface. 





[prev-post]: {{site.baseurl}}/{% post_url 2025-06-19-obtaining-the-list-of-peers %}
[torrent-file-0.0.4]: https://github.com/tindandelion/rust-bittorrent-client/blob/0.0.4/test-data/debian-12.11.0-amd64-netinst.iso.torrent
[wiki-tracker-response]: https://wiki.theory.org/BitTorrentSpecification#Tracker_Response
[raw-tracker-response]: {{site.baseurl}}/{% post_url 2025-06-19-obtaining-the-list-of-peers %}#make-it-run