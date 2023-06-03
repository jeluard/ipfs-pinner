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

## License

[MIT](https://choosealicense.com/licenses/mit/)