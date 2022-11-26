use {log::*, url::Url, anyhow::{*, Result}, futures::future::FutureExt, dotenv, subslice::SubsliceExt};
use async_std::{ net::{SocketAddr, TcpListener, TcpStream},	prelude::*,	task};

const BUFFER_SIZE: usize = 1024;

#[async_std::main]
async fn main() -> Result<()> {
	env_logger::init();
	dotenv::dotenv().ok();
	let server_address = dotenv::var("PROXY_ADDRESS").unwrap_or("0.0.0.0:8088".to_owned());
	let server = TcpListener::bind(&server_address).await.unwrap();
	info!("listening on {}", &server_address);
	while let Result::Ok((client_stream, client_addr)) = server.accept().await {
		task::spawn(async move {
			match process_client(client_stream, client_addr).await { anyhow::Result::Ok(()) => (), Err(e) => error!("error: {}", e), }
		});
	}
	Ok(())
}

async fn process_client(mut client_stream: TcpStream, client_addr: SocketAddr) -> Result<()> {
	let mut buf : [u8; BUFFER_SIZE] = unsafe { std::mem::MaybeUninit::uninit().assume_init() }; 
	let mut c = 0;
	let (count, index) = loop {
		let readed = client_stream.read(&mut buf[c..]).await?;
		if readed == 0 { return Ok(()); }
		c += readed;
		match buf.find(&[13, 10, 13, 10]) {
			Some(pos) => break (c, pos),
			None => if c < BUFFER_SIZE { // didn't read full request head yet, read more;
				error!("not find head end yet, read more");
				continue;
			} else{
				let request = String::from_utf8_lossy(&buf[..c]);
				return  Err(anyhow!("read {} bytes, buffer almost full, but can't find end!\n {}", c, request));
			}
		};
	};
	
	let request = String::from_utf8_lossy(&buf[..index]);
	let mut lines = request.lines();

	let Some(line) = lines.next() else { return Err(anyhow!("get request line failed!"));};
	
	let mut fields = line.split_whitespace();
	let Some(method) = fields.next() else { return Err(anyhow!("can't find request method"))};
	let Some(url_str) = fields.next() else { return Err(anyhow!("can't find url"))};

	let (https, address) = match method {
		"CONNECT"  => (true, String::from(url_str)),
		_ => {
			let url =  Url::parse(url_str)?;
			match url.host() {
				Some(addr) => (false, format!("{}:{}", addr.to_string(), url.port().unwrap_or(80))),
				_ => return Err(anyhow!("can't find host from url")),
			}
		}
	};

	info!("{} -> {}", client_addr.to_string(), line);

	let server_stream = TcpStream::connect(address).await?;

	let (local_reader, local_writer) = &mut (&client_stream, &client_stream);
	let (server_reader, server_writer) = &mut (&server_stream, &server_stream);

	if https { local_writer.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await?;} 
	else { server_writer.write_all(&buf[..count]).await?; }

	let copy_task_tx = async_std::io::copy(local_reader, server_writer);
	let copy_task_rx = async_std::io::copy(server_reader, local_writer);

	let _ = futures::select! { r1 = copy_task_tx.fuse() => r1, r2 = copy_task_rx.fuse() => r2 }?;
	
	Ok(())
}