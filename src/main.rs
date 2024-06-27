use std::io;
use tokio::{net::{TcpListener, TcpStream}, sync::broadcast::{self, Sender}};

use chat_server::{handle, lib::Names,lib::Rooms};
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 创建一个 Tcp Listener，监听 127.0.0.1:42069
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    let names =Names::new();
    let rooms = Rooms::new();
    loop {
        let (tcp, _) = server.accept().await?;
        tokio::spawn(handle(tcp, rooms.clone(),names.clone()));
    }
}
