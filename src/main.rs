
use async_std::{
	net::{TcpListener, TcpStream},
	prelude::*,
	task,
};
use futures::future::FutureExt;
use std::io;
use std::time::Instant;
use url::Url;
use log::*;

#[async_std::main]
async fn main() -> std::io::Result<()> {
	env_logger::init();
	let server_address = "0.0.0.0:8088";
	let server = TcpListener::bind(server_address).await.unwrap();
	info!("listening on {}", server_address);
	while let Ok((client_stream, client_addr)) = server.accept().await {
		debug!("accept client: {}", client_addr);
		task::spawn(async {
			//let start = Instant::now();
			match process_client(client_stream).await {
				Ok(()) => (),
				Err(e) => error!("error: {}", e),
			}
			//debug!("finished in {} ms.", start.elapsed().as_millis())
		});
	}
	Ok(())
}

enum Method {
	CONNECT,
	OTHER,
}

async fn process_client(mut client_stream: TcpStream) -> io::Result<()> {
	let client_addr = &client_stream.peer_addr()?;
	// Read the CONNECT request
	let mut buf = [0; 4096];
	let mut count = 0;
	loop {
		let bytes = client_stream.read(&mut buf[count..]).await?;
		if bytes > 0 {
			count += bytes;
			if count > 8 {
				break;
			}
		}
	};

	let request;
	let mut index = 0;
	for i in buf.iter() {
		if *i == '\n' as u8 {
			break;
		}
		if index <= count {
			index += 1;
		} else{
			break
		}
	}

	if index < 3 {
		return Err(io::Error::new(io::ErrorKind::Other, "request to short"));
	}

	debug!("index: {:?}", index);

	match std::str::from_utf8(&buf[..index]){
	Ok(s) => {
			request = String::from(s);
		},
		Err(e) => {
			error!("error: {:?}", e);
			return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
		}
	}
	
	let mut address : String;
	let v: Vec<&str> = request.split(' ').collect();
	if v.len() < 2 {
		error!("bad request: {}", request);
		return Err(io::Error::new(io::ErrorKind::Other, "failed parse request"));
	}

	info!("{} -> {}", client_addr.to_string(), request);

	address = String::from(v[1]);
	let method = if v[0] == "CONNECT" {
		debug!("CONNECT address: {}", address);
		Method::CONNECT
	} else {
		debug!("{} address: {}",v[0], address);
		if address.starts_with("http://") {
 			match Url::parse(v[1]) {
				Ok(url) => {
					let addr = url.host().unwrap();
					let port :u16 = match url.port(){Some(p) => p,	None => 80,};
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
	let server_stream = TcpStream::connect(address,).await?;

	let (local_reader, local_writer) = &mut (&client_stream, &client_stream);
	let (server_reader, server_writer) = &mut (&server_stream, &server_stream);

	match method {
		Method::CONNECT => {
			local_writer.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await?;
		} ,
		_ => {
			server_writer.write_all(&buf).await?;
		},
	}
	
    let copy_task_a = async_std::io::copy(local_reader, server_writer);
    let copy_task_b = async_std::io::copy(server_reader, local_writer);
	
    let _ = futures::select! {
        r1 = copy_task_a.fuse() => r1,
        r2 = copy_task_b.fuse() => r2
    };

    Ok(())
	
}
