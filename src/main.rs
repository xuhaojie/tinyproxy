
use async_std::net::{TcpListener, TcpStream};
use async_std::prelude::*;
use futures::future::FutureExt;
use std::io;
use async_std::task;
use url::{Host, Origin, Url};


#[async_std::main]
async fn main() -> std::io::Result<()> {
	let server = TcpListener::bind("127.0.0.1:8088").await.unwrap();
	while let Ok((client_stream, client_addr)) = server.accept().await {
		println!("accept client: {}", client_addr);
		// 每接入一个客户端的连接请求，都分配一个子任务，
		// 如果客户端的并发数量不大，为每个客户端都分配一个thread，
		// 然后在thread中创建tokio runtime，处理起来会更方便

		task::spawn(async {
			process_client(client_stream).await;
		});
	}
	Ok(())
}

async fn process_client(mut client_stream: TcpStream) -> io::Result<()> {
	let addr = &client_stream.local_addr()?;
	println!("client addr:{:?}",addr);
	// Read the CONNECT request
	let mut buf = [0; 4096];
	let nbytes = client_stream.read(&mut buf).await?;

	let mut req = String::from("");
	match std::str::from_utf8(&buf[..nbytes]){
	Ok(s) => {req = String::from(s)},
	Err(e) => {
		println!("error: {:?}", e);
		
	}
	}


	println!("Received request {}", req);

	let mut address : String;
	let v: Vec<&str> = req.split(' ').collect();
	if v[0] == "CONNECT" {
		println!(":CONNECT {}", v[1]);
		address = String::from(v[1]);

	} else {
		println!(":{} {}", v[0], v[1]);
		address = String::from(v[1]);
		println!("{}", address);
		
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

/*
			let tmp_str = String::from(address).replace("http://", "");
			let ac = tmp_str.chars().count();
	
			let mut new_tmp_str = tmp_str[..ac-1].to_owned(); 
			new_tmp_str.push_str(":80");
			address = new_tmp_str;
*/			
			println!("address: {}", address);
		}


	}

   client_stream.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await?;

	// Connect to a peer
	
	
	//let mut server_stream = TcpStream::connect(address,).await?;
	let mut server_stream = TcpStream::connect(address,).await?;

	let (lr, lw) = &mut (&client_stream, &client_stream);
	let (tr, tw) = &mut (&server_stream, &server_stream);

//	tw.write(action.as_bytes());

    let copy_a = async_std::io::copy(lr, tw);
    let copy_b = async_std::io::copy(tr, lw);
	
    let r = futures::select! {
        r1 = copy_a.fuse() => r1,
        r2 = copy_b.fuse() => r2
    };

    Ok(())
	
}
