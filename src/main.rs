use url::Url ;
use std::net::{SocketAddr};
use base64::{encode, decode};
use tokio::io::{AsyncWriteExt, Result};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use fast_log::{init,Config};
use log::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    fast_log::init(Config::new().file("./tinyproxy.log")).unwrap();
	let server_address = "0.0.0.0:8010";
	let server = TcpListener::bind(server_address).await.unwrap();
	info!("listening on {}", server_address);
		while let Ok((client_stream, client_addr)) = server.accept().await {
			tokio::spawn(async move { 
				match process_client(client_stream, client_addr.clone()).await{
				Ok(())=>{info!("process_client ok:{}",client_addr); }

				Err(e)=>{info!("process_client error: {}", e); }
				}
			});
		}
	Ok(())
}

async fn process_client(mut client_stream: TcpStream, client_addr: SocketAddr) ->Result<()> {
	let mut buf = Vec::with_capacity(1024);
	client_stream.readable().await?;
	let count = match client_stream.try_read_buf(&mut buf)
		{
            Ok(0) => { return Ok(()); }
            Ok(n) => { n }
            Err(e) => {
				
                return Err(e.into());
            }
        };
	let request = String::from_utf8_lossy(&buf);
	let mut auth=false;
	let mut authcode :String="".to_string();
  	let mut authcode=base64::encode(b"tzf:1234");
	 if  None!=request.find(authcode.as_str()){
		auth=true;	
	}
	
	if auth==false{
		client_stream.write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm=\"*\"\r\n\r\n").await?;
		 return Ok(())
	}
	let line = request.lines().next().unwrap_or("");
	let fields: Vec<&str> = line.split_whitespace().collect();
	if fields.len() < 2 { return Ok(()); }
	let method = fields[0];
	let mut address0 ="".to_string();
	match Url::parse(fields[1]){
		Ok(t)=>{if let Some(addr) = t.host(){
			let port: u16 = t.port().unwrap_or(80);
			address0=format!("{}:{}", addr.to_string(), port);
		}
		}
		Err(e)=>{ info!("Url::parse error: {}",e);
		return Ok(())
		}
	};
	let (https, address) = if method == "CONNECT" {
		(true, String::from(fields[1]))
	} else {
		(false,address0)
	};
	let mut server_stream = TcpStream::connect(address).await?;

	if https { client_stream.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").await?;} 
	else { server_stream.write_all(&buf[..count]).await?; }

    	tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await?;

	Ok(())
}
