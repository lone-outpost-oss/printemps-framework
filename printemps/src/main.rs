//! The Runner of the [Printemps Web Framework](https://www.printempsframework.org/).
//! 
//! ## Intro
//! 
//! This application is the runtime for running a Printemps web application.
//! 
//! See `README.md` and other docs for detailed usage.
//! 
//! ## For Printemps users
//! 
//! This is just a runner, not the framework itself. Most users don't have to deal with this repository.
//!  
//! To build a Printemps web application, see corresponding packages/repositories in MoonBit language (not ready yet).
//! 
//! ## WARNING
//! 
//! This software is HIGHLY EXPERIMENTAL and won't reach even `0.1` in a short period. 
//! 
//! __ANY API IS SUBJECT TO CHANGE, USE AT YOUR OWN RISK.__

use anyhow::anyhow;
use anyhow::Result;

type BoxBodyResponse = hyper::Response<http_body_util::combinators::BoxBody<bytes::Bytes, anyhow::Error>>;

mod cmdline;
mod prelude;

async fn hello(
    req: hyper::Request<hyper::body::Incoming>,
) -> Result<BoxBodyResponse> {
    use http_body_util::BodyExt;
    let bytes = req.into_body().collect().await?.to_bytes();

    let rsp_body = http_body_util::Full::new(bytes).map_err(|_| anyhow!("unreachable")).boxed();
    Ok(hyper::Response::new(rsp_body))
}

/// Application entrypoint.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use tokio::net::TcpListener;
    use std::{
        net::{IpAddr, SocketAddr},
        str::FromStr,
    };    

    let args = cmdline::Args::parse();
    dbg!(&args);

    let listen_ip = IpAddr::from_str(&args.listen_addr)?;
    let listen_addr = SocketAddr::from((listen_ip, args.listen_port));

    let listener = TcpListener::bind(listen_addr).await?;

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                println!("accepting connection from {}", &addr);
                let io = hyper_util::rt::TokioIo::new(stream);
                tokio::task::spawn(async move {
                    use hyper::service::service_fn;
                    use hyper_util::rt::TokioTimer;

                    if let Err(err) = hyper::server::conn::http1::Builder::new()
                        .timer(TokioTimer::new())
                        .serve_connection(io, service_fn(hello))
                        .await
                    {
                        eprintln!("Error serving connection: {:?}", err);
                    }
                });
            }
            Err(err) => {
                eprintln!("error accepting incoming connection: {}", &err);
            }
        }
    }
}
