mod utils;

use base64::{engine::general_purpose, Engine as _};
use bytes::Bytes;
use http::{HeaderValue, Method, Uri};
use http_body_util::{combinators::BoxBody, BodyExt};
use hyper::{
    body::Incoming, client::conn::http1::Builder, server::conn::http1, service::service_fn,
    Request, Response,
};
use sp_core::{
    crypto::{AccountId32, Ss58Codec},
    sr25519::Public,
};
use std::{collections::HashMap, env, error::Error, net::SocketAddr, str};
use subxt::{OnlineClient, PolkadotConfig};
use tokio::{
    net::{TcpListener, TcpStream},
    task::spawn,
};
use utils::{
    hyper::{accepted, bad_request},
    polkadot::{balance, verify},
    Result,
};

#[tokio::main]
async fn main() -> Result<()> {
    let chain_rpc = match env::var("CHAIN_RPC") {
        Ok(value) => value,
        Err(_) => return Err("Missing CHAIN_RPC env variable".into()),
    };
    let token_min = match env::var("TOKEN_MIN") {
        Ok(value) => value.replace('_', "").parse::<u128>()?,
        Err(_) => return Err("Missing TOKEN_MIN env variable".into()),
    };
    let port = env::var("PORT")
        .unwrap_or_else(|_| "5002".to_string())
        .parse::<u16>()?;
    let remote_addr = env::var("IPFS_DAEMON").unwrap_or_else(|_| "localhost:5001".to_string());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await?;

    println!(
        "Listening on http://{addr}, proxying to {remote_addr}\nUsing rpc {chain_rpc} with a token limit of {token_min}",
    );

    loop {
        let (stream, _) = listener.accept().await?;
        let local_chain_rpc = chain_rpc.clone();
        let local_remote_addr = remote_addr.clone();

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .title_case_headers(true)
                .serve_connection(
                    stream,
                    service_fn(|req| {
                        proxy(
                            req,
                            local_chain_rpc.clone(),
                            local_remote_addr.clone(),
                            token_min,
                        )
                    }),
                )
                .with_upgrades()
                .await
            {
                println!("Failed to serve connection: {err:?}");
            }
        });
    }
}

const AUTHORIZATION: &str = "authorization";

fn normalize_hex(hex: &str) -> &str {
    hex.trim_start_matches("0x")
}

fn extract_args(uri: &Uri) -> HashMap<String, String> {
    uri
        .query()
        .map(|v| {
            url::form_urlencoded::parse(v.as_bytes())
                .into_owned()
                .collect()
        })
        .unwrap_or_else(HashMap::new)
}

struct Parts {
    address: String,
    signature: String,
}

fn extract_parts(auth: &HeaderValue) -> Result<Option<Parts>> {
    let auth = auth.to_str()?.replace("Bearer", "");
    let bytes = general_purpose::STANDARD.decode(auth.trim())?;
    let str = str::from_utf8(&bytes)?;
    let parts: Vec<&str> = str.split(':').collect();
    if let [address, signature] = parts[..] {
        let address = normalize_hex(address).to_string();
        let signature = normalize_hex(signature).to_string();
        return Ok(Some(Parts { address, signature }));
    }
    Ok(None)
}

async fn validate_account_requirements(
    api: &OnlineClient<PolkadotConfig>,
    address: String,
    token_min: u128,
) -> Result<()> {
    let account = AccountId32::from_ss58check(&address)?;
    let value = balance(api, account).await?;
    if let Some(value) = value {
        if value < token_min {
            return Err("Balance too low".into());
        }
    } else {
        return Err("No balance".into());
    }
    Ok(())
}

fn is_request_allowed(req: &Request<Incoming>) -> bool {
    if req.method() != Method::POST {
        return false;
    }

    let path = &req.uri().path()[1..]; // strip first '/' from path
    let parts: Vec<&str> = path.split_terminator('/').collect();
    parts[0] == "api"
        && parts[1] == "v0"
        && parts[2] == "pin"
        && (parts[3] == "add"
            || parts[3] == "ls"
            || parts[3] == "rm"
            || parts[3] == "update"
            || parts[3] == "verify")
}

async fn proxy_request(
    addr: String,
    req: Request<Incoming>,
) -> std::result::Result<Response<BoxBody<Bytes, hyper::Error>>, Box<dyn Error + Send + Sync>> {
    let stream = match TcpStream::connect(addr).await {
        Ok(value) => value,
        Err(_) => return Ok(bad_request("Failed to connect to proxy".to_string())),
    };

    let (mut sender, conn) = Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .handshake(stream)
        .await?;

    spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {err:?}");
        }
    });

    let resp = match sender.send_request(req).await {
        Ok(value) => value,
        Err(_) => return Ok(bad_request("Failed to send request to proxy".to_string())),
    };
    Ok(resp.map(|b| b.boxed()))
}

async fn proxy(
    req: Request<hyper::body::Incoming>,
    chain_rpc: String,
    addr: String,
    token_min: u128,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>> {
    // Answer back CORS pre-flight requests
    if req.method() == Method::OPTIONS {
        return Ok(accepted());
    }

    if !(is_request_allowed(&req)) {
        return Ok(bad_request("Incorrect api call".to_string()));
    }

    if let Some(auth) = req.headers().get(AUTHORIZATION) {
        // Extracts address and signature from AUTHORIZATION header
        let Parts { address, signature } = match extract_parts(auth) {
            Ok(value) => match value {
                Some(value) => value,
                None => return Ok(bad_request(("Failed to parse extracted parts").to_string())),
            },
            Err(err) => return Ok(bad_request(format!("Failed to extract parts: {err:?}"))),
        };

        // Get CID from URL argument
        let args = extract_args(req.uri());
        let cid = match args.get("arg") {
            Some(value) => value,
            None => return Ok(bad_request("Failed to extract CID".to_string())),
        };

        // Verify provided signature is correct
        let decoded_signature = match hex::decode(signature.clone()) {
            Ok(value) => value,
            Err(_) => return Ok(bad_request("Failed to decode signature".to_string())),
        };
        let message = format!("<Bytes>{address}/{cid}</Bytes>");
        let public_key = match Public::from_ss58check(&address) {
            Ok(value) => value,
            Err(_) => return Ok(bad_request("Failed to parse address".to_string())),
        };
        if !verify(&decoded_signature[..], message.as_bytes(), &public_key) {
            return Ok(bad_request("Incorrect signature".to_string()));
        }

        // Ensure signing account matches requirements
        match &OnlineClient::<PolkadotConfig>::from_url(chain_rpc.clone()).await {
            Ok(api) => {
                if let Err(err) = validate_account_requirements(api, address, token_min).await {
                    return Ok(bad_request(err.to_string()));
                }
            }
            Err(_) => {
                return Ok(bad_request(format!(
                    "Failed to connect to remote rpc {chain_rpc}"
                )))
            }
        };
    } else {
        return Ok(bad_request(
            "Missing required authorization header".to_string(),
        ));
    }

    proxy_request(addr, req).await
}
