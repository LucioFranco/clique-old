# Roadmap

## Library - clique

- [x] Transport
  - [x] UDP Framed
  - [x] gRPC TCP/h2
- [ ] SWIM
  - [ ] Messages
	  - [x] Ping
	  - [x] Ack
	  - [ ] PingReq
	  - [ ] PingReqAck (better name for this?)
  - [ ] Broadcasts
	- [x] Join
	- [ ] Leave
	- [ ] Alive
	- [ ] Suspect
	- [ ] Dead
  - [ ] Piggybacking UDP Gossip messages
  - [ ] Gossip
  	- [x] Gossip on configured interval
	- [ ] `> 3` Cluster pick `k` random nodes to probe
  -[ ] RPC
	- [x] Join Push/Pull
	- [ ] Occasional Push/Pull requests
	- [ ] Client RPC
		- [ ] Join
		- [ ] Members
		- [ ] Leave
		- [ ] Research more
- [ ] Public API
  - [x] Join
  - [x] Serve UDP/TCP
  - [ ] Event channel
  - [x] Peers snapshot
- [ ] Custom Event Dissemination
- [ ] Node metadata
  - [ ] Binary blob data `Vec<u8>`
  - [ ] Tags
  
## CLI - clique-agent
- [ ] Clap for CLI
- [ ] Commands
  - [ ] Run
  - [ ] Members/Peers list
  - [ ] Leave
  - [ ] Join
- [ ] RPC
  - [ ] Join
  - [ ] Members
  - [ ] Leave
