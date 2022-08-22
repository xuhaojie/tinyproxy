use async_std::{
	net::{TcpListener, TcpStream},
	prelude::*,
	task,
};
use futures::future::FutureExt;
use log::*;
use std::io;
use url::Url;

#[async_std::main]
async fn main() -> std::io::Result<()> {
	env_logger::init();
	let server_address = "0.0.0.0:8088";
	let server = TcpListener::bind(server_address).await.unwrap();
	info!("listening on {}", server_address);
	while let Ok((client_stream, client_addr)) = server.accept().await {
		debug!("accept client: {}", client_addr);
		task::spawn(async {
			match process_client(client_stream).await {
				Ok(()) => (),
				Err(e) => error!("error: {}", e),
			}
		});
	}
	Ok(())
}

async fn process_client(mut client_stream: TcpStream) -> io::Result<()> {
	let client_addr = &client_stream.peer_addr()?;
	let mut buf = [0; 1024];
	
	let count = client_stream.read(&mut buf).await?;
	if count == 0 {
		return Ok(());
	}

	let mut lines = buf.split(|c| *c == '\n' as u8);
	let request = match lines.next() {
		Some(line) => match std::str::from_utf8(line) {
			Ok(req) => req.to_owned(),
			_ => return Err(io::Error::new(io::ErrorKind::Other,"bad utf-8 string")),
		},
		_ => return Err(io::Error::new(io::ErrorKind::Other,"failed split lines")),
	};

	let fields: Vec<&str> = request.split(' ').collect();
	if fields.len() < 2 {
		return Err(io::Error::new(io::ErrorKind::Other,"bad request"));
	}

	info!("{} -> {}", client_addr.to_string(), request);

	let method = fields[0];
	let url = fields[1];
	let (https, address) = if method == "CONNECT" {
		(true, String::from(url))
	} else {
		(
			false,
			match Url::parse(url) {
				Ok(url) => {
					if let Some(addr) = url.host() {
						let port: u16 = match url.port() { Some(p) => p, None => 80, };
						format!("{}:{}", addr.to_string(), port)
					} else {
						return Err(io::Error::new(io::ErrorKind::Other, "bad host in url"));
					}
				}
				_ => return Err(io::Error::new(io::ErrorKind::Other, format!("failed to parse url: {}", fields[1]))),
			}
		)
	};

	debug!("{} address: {}", method, address);

	let server_stream = TcpStream::connect(address).await?;

	let (local_reader, local_writer) = &mut (&client_stream, &client_stream);
	let (server_reader, server_writer) = &mut (&server_stream, &server_stream);

	if https {
		local_writer.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await?;
	} else {
		server_writer.write_all(&buf[..count]).await?;
	}

	let copy_task_a = async_std::io::copy(local_reader, server_writer);
	let copy_task_b = async_std::io::copy(server_reader, local_writer);

	let _ = futures::select! {
		r1 = copy_task_a.fuse() => r1,
		r2 = copy_task_b.fuse() => r2
	};

	Ok(())
}
