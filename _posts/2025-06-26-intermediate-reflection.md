---
layout: post
title:  "Time for reflection"
date: 2025-06-26
---

I've made some progress with communicating with the torrent tracker so far, and I'm ready to dive into the details of peer-to-peer communication. However, I have some doubts about what problem to tackle next. Should I dive into the technical details of peer-to-peer communication, or should I polish the code I already have? Or maybe I should look into areas I haven't even touched yet? I'd like to step back and look at the bigger picture. 

# What we've got so far 

Let's make a quick recap of what we've got so far: 

* We can parse the torrent file and extract data from it. It looks like we can access all information from the torrent file, but we only use the bits that are needed to make the initial announce request to the torrent tracker. 
* We can make a request to the torrent tracker and get a success response. We supply only the minimum amount of information in the announce request, just enough for the tracker to respond with the peer list. Also, we're not very robust when it comes to handling errors during that exchange. 
* We can parse the tracker response and extract the information about peers: their IP addresses and ports. That's enough to start communicating with them. 

# The components of the BitTorrent client 

Based on my current understanding, the BitTorrent client consists of a few key components. Here I've sketched out a rough picture of my current vision. 

![BitTorrent client components]({{ site.baseurl }}/assets/images/intermediate-reflection-1/bt-client-components.svg)

Let's discuss what we have here. The description is going to be rather vague because I'm still learning about the technical details of BitTorrent implementation. It kind of maps out what I've learned so far and may very well be incorrect. 

First, _file downloaders_ are responsible for downloading file pieces from the peers. In my mind, there's a pool of such entities to handle downloads in parallel. They use the information from the tracker file to carry out the operations: piece length, piece hashes, etc. 

The _file uploader_ is a component that handles incoming download requests from peers. I'm not sure if there should be one such uploader or many. Let's assume, for simplicity, that there will be just one such uploader. I guess it will also need access to the information from the tracker file to handle the requests. 

The _tracker updater_ is a piece that handles communication with the torrent tracker. As far as I understand, it should make requests to the tracker periodically to send current download stats (bytes uploaded, bytes downloaded). It also receives the updated list of peers from the tracker, so it probably will need to pass this information to the downloader pool, for example, to try connecting to new peers. 

Finally, there's a _user interface_ component. I drew it a bit on the side and didn't connect it to anything yet because a) it's a component that requires a lot of exploration, and b) potentially, it will need some information from all other components. On the other hand, I would like to isolate the user interface from the core of the application. 

# Sketching the user interface 

And here's my very ambitious vision for the user interface: 

![BitTorrent client user interface]({{ site.baseurl }}/assets/images/intermediate-reflection-1/bt-client-ui.svg)

I'm not sure if it's implementable or how much effort it may take to implement. I need to explore what libraries for building UIs in Rust are available and how they could help. There's a big chance I'll need to scale down my ambitions. After all, the focus of this project is to build a core BitTorrent client, not fancy UI stuff. If it turns out to be too complicated, I'll sacrifice it without a doubt. Still, it would be nice to have some kind of user-friendly UI. Time will show if this vision turns into reality. 

# My reflections 

Looking at the scope of the project, a few thoughts come to my mind: 

* I'm quite confident about the _tracker updater_ component. In a nutshell, it's a straightforward piece. We just need to make GET requests on a schedule and pass updated information in the request parameters. I already know how to do it. It also means I can leave the current code as-is, in its very imperfect condition. I'm going to come back and polish it when the time comes to implement the tracker updater properly. 

* The _file downloader_ is the most uncertain component now, but it's also the centerpiece of the entire application. If I don't make it right, nothing else matters. **By all means, that should be my top priority**. As a starting point, I can start with having just a single downloader, make it work, and then scale up to multiple downloaders working in parallel. 

* Another piece with great uncertainty is the _user interface_. However, the user interface is secondary -- I can live without it. As a fall-back solution, I can just print out in the console some kind of an event log to have some visibility about what's going on. 

# The plan 

I've tried to break down the work into sizable and meaningful user stories and lay them down in the order of priority. Some stories I consider required, others I'd mark as nice-to-have, as long as I still have time and interest in this project. 

#### Required stories

* _Connect to the peers_. Connect to a single peer from the list returned by the torrent tracker and keep the connection alive. Nothing else is required at this step. 
* _Graceful program termination_. Let's add in some good housekeeping. If the user terminates the program by hitting `Ctrl+C`, we should end the program gracefully, meaning: 
    - Gracefully disconnect from the connected peer; 
    - Send a notification to the tracker that the download has stopped; 

    In my experience, it's better to take care of such things early in the process. 
* _Download and store a single file piece from the peer_. The outcome of this story is that I should become familiar with the protocol. 
* _Download the entire file from a single peer_. If the user interrupts the download, next time the application should start from a blank slate. When the download is complete, we should notify the tracker and exit the program. 
* _Download the file from multiple peers in parallel_.

#### Nice-to-have stories 

* _User interface_. Explore the possibilities to build a UI close enough to my "ideal" sketch. 
* _Resume the partially downloaded file_. If the user interrupted the program, then next time it starts, the download should be resumed from where it was interrupted. 
* _Tracker updates_. Periodically update the tracker with the current download progress. 
* _Handle upload requests_. That implies that the program keeps running even after the file is fully downloaded. The program serves upload requests from peers as a seeder.


    




