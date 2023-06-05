# IPFS pinner

IPFS pinner is a decentralized IPFS pinner, acting as a proxy on top of a regular IPFS daemon.
It relies on polkadot relay-chains to provide a permissionless pinning gateway. This allows end-users to pin files in client environment (e.g. browser) provided their address has sufficient funds.

## Installation

Clone the repo, compile and execute.

```bash
cargo run
```

## Usage

First start an IPFS daemon with the appropriate headers. It should probably be hidden behind a proxy.

```shell
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Methods '["PUT", "GET", "POST", "OPTIONS"]'
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Origin "[\"*\"]"
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Headers '["authorization"]'

ipfs daemon
```

Then start the pinner service. It will connect to the local IPFS daemon.

```shell
IPFS_DAEMON=localhost:5001 CHAIN_RPC=wss://kusama-rpc.polkadot.io:443 TOKEN_MIN=1_000_000 cargo run
```

### API

IPFS pinner only exports `api/v0/pin/*` endpoints. In particular, file upload should be handled by other means.

An example of its usage can be found in the [example folder](example/)

### Full deployment

A full prod-like deployment would be:

* `ipfs-pinner` open on internet (behind a regular HTTP proxy)
* [kubo](https://github.com/ipfs/kubo) partially hidden (at least admin and pin methods); kubo is still used to add files

## Architecture

At high-level, the flow is:

* listen for pin HTTP endpoints (same API as a regular IPFS node)
* per pin call, extract a polkadot address and a signature from HTTP headers. Signature is address/CID
* verify the signature is valid with regard to the address
* verify the signed CID is the one being pinned
* verify the address holds enough DOTs regarding all their pinned CIDs (not fully implemented yet)
* if everything is true, proxy the call to the real IPFS node

### DOS protection

Disk space quotas per address pro-rata of the amount of tokens they hold are enforced.
A background process then removes files associated with address that don’t have those funds anymore.

Note that most of this logic hasn't been implemented yet.

## License

[MIT](https://choosealicense.com/licenses/mit/)