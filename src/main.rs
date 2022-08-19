
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

async fn process_client(mut client_stream: TcpStream) -> io::Result<()> {
	let client_addr = &client_stream.peer_addr()?;
	// Read the CONNECT request
	let mut buf = [0; 1024];

	let count = client_stream.read(&mut buf).await?;
	if count == 0 {
		return Ok(());
	}
	
	let parse_error = Err(io::Error::new(io::ErrorKind::Other, "failed parse request"));

	let mut lines = buf.split(|c| *c == '\n' as u8);
	let request = if let Some(line) = lines.next(){
		if let Ok(req) = std::str::from_utf8(line){
			req.to_owned()
		} else {
			"".to_owned()
		}
	} else {"".to_owned()};

	let fields: Vec<&str> = request.split(' ').collect();
	if fields.len() < 2 {
		error!("bad request: {}", request);
		return parse_error;
	}

	info!("{} -> {}", client_addr.to_string(), request);
	let mut address = String::from(fields[1]);
	let method  = fields[0];
	let connect = if method == "CONNECT" {
		true
	} else {
		match Url::parse(fields[1]) {
			Ok(url) => {
				let addr = url.host().unwrap();
				let port :u16 = match url.port(){Some(p) => p,	None => 80,};
				address = format!("{}:{}",addr.to_string(), port)
			}
			Err(err) => println!("{}", err),
		}
		false
	};
	debug!("{} address: {}", method, address);
	// Connect to a peer
	let server_stream = TcpStream::connect(address,).await?;

	let (local_reader, local_writer) = &mut (&client_stream, &client_stream);
	let (server_reader, server_writer) = &mut (&server_stream, &server_stream);

	if connect {
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
