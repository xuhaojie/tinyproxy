
use async_std::{
	net::{TcpListener, TcpStream},
	prelude::*,
	task,
};
use futures::future::FutureExt;
use std::io;
use url::Url;
use log::*;

#[async_std::main]
async fn main() -> std::io::Result<()> {
	env_logger::init();
	let server_address = "0.0.0.0:8088";
	let server = TcpListener::bind(server_address).await.unwrap();
	info!("listening on {}", server_address);
	while let Ok((client_stream, client_addr)) = server.accept().await {
		info!("accept client: {}", client_addr);
		task::spawn(async {
			process_client(client_stream).await;
		});
	}
	Ok(())
}

enum Method {
	CONNECT,
	OTHER,
}

async fn process_client(mut client_stream: TcpStream) -> io::Result<()> {
	let addr = &client_stream.peer_addr()?;
	info!("client addr:{:?}",addr);
	// Read the CONNECT request
	let mut buf = [0; 256];
	let bytes = client_stream.read(&mut buf).await?;

	let mut request;
	match std::str::from_utf8(&buf[..bytes]){
		Ok(s) => {request = String::from(s)},
		Err(e) => {
			error!("error: {:?}", e);
			return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
		}
	}

	let mut address : String;
	let v: Vec<&str> = request.split(' ').collect();
	if v.len() < 2 {
		io::Error::new(io::ErrorKind::Other, "failed parse request");
	}
	let method = if v[0] == "CONNECT" {
		address = String::from(v[1]);
		info!(":CONNECT address: {}", address);
		Method::CONNECT
	} else {
		address = String::from(v[1]);
		if address.starts_with("http://"){
 
			match Url::parse(v[1]) {
				Ok(url) => {
					let addr = url.host().unwrap();
					address = addr.to_string();
					address.push_str(":80");
					address.push_str(":");
					let mut port :u16 = 80;
					match url.port(){
						Some(p) => {
							port = p;
						},
						None => {
					
						}
					}
					address = format!("{}:{}",addr.to_string(), port)
				}
				Err(err) => println!("{}", err),
			}
			debug!("address: {}", address);
		} else{
			error!("???? address: {}", address);
		}
		Method::OTHER
	};

	// Connect to a peer
	let mut server_stream = TcpStream::connect(address,).await?;

	let (lr, lw) = &mut (&client_stream, &client_stream);
	let (tr, tw) = &mut (&server_stream, &server_stream);

	match method {
		Method::CONNECT => {
			lw.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await?;
		} ,
		_ => {
			tw.write_all(&buf).await?;
		},
	}
	
    let copy_a = async_std::io::copy(lr, tw);
    let copy_b = async_std::io::copy(tr, lw);
	
    let r = futures::select! {
        r1 = copy_a.fuse() => r1,
        r2 = copy_b.fuse() => r2
    };

    Ok(())
	
}
