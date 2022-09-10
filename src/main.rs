use async_std::{ net::{SocketAddr, TcpListener, TcpStream},	prelude::*,	task,};
use futures::future::FutureExt;
use log::*;
use url::Url;

use anyhow::{*,Result};

#[async_std::main]
async fn main() -> Result<()> {
	env_logger::init();
	let server_address = "0.0.0.0:8088";
	let server = TcpListener::bind(server_address).await.unwrap();
	info!("listening on {}", server_address);
	while let Result::Ok((client_stream, client_addr)) = server.accept().await {
		debug!("accept client: {}", client_addr);
		task::spawn(async move {
			match process_client(client_stream, client_addr).await {
				anyhow::Result::Ok(()) => (),
				Err(e) => error!("error: {}", e),
			}
		});
	}
	Ok(())
}
  
async fn process_client(mut client_stream: TcpStream, client_addr: SocketAddr) -> Result<()> {
	let mut buf = [0; 1024];
	let count = client_stream.read(&mut buf).await?;
	if count == 0 {
		return Ok(());
	}

	let request = String::from_utf8_lossy(&buf);
	let line = request.lines().next().unwrap_or("");
	let fields: Vec<&str> = line.split_whitespace().collect();
	if fields.len() < 2 {
		return Err(anyhow!("bad request"));
	}

	info!("{} -> {}", client_addr.to_string(), line);

	let method = fields[0];
	let url = Url::parse(fields[1])?;
	let (https, address) = if method == "CONNECT" {
		(true, String::from(fields[1]))
	} else {(
		false,
		if let Some(addr) = url.host() {
			let port: u16 = url.port().unwrap_or(80);
			format!("{}:{}", addr.to_string(), port)
		} else {
			return Err(anyhow!( "bad host in url"));
		}
	)};

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
