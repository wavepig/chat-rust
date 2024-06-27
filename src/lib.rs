use std::{collections::HashSet, sync::{Arc, Mutex}};

use futures::{SinkExt, StreamExt};
use tokio::{net::{TcpListener, TcpStream}, sync::broadcast::{self, Sender, Receiver}};
use tokio_util::codec::{Decoder, Encoder, FramedRead, FramedWrite, LengthDelimitedCodec, LinesCodec, LinesCodecError};
use bytes::{Buf, BufMut, BytesMut};

#[path ="shared/lib.rs"]
pub mod lib;
use lib::{b, Names,Rooms, length_code::LengthPrefix,MAIN};

const HELP_MSG: &str = include_str!("docs/help.txt");

pub async fn handle(mut tcp: TcpStream, rooms: Rooms, names: Names) -> anyhow::Result<()> {
    let (reader, writer) = tcp.split();
    let mut stream = FramedRead::new(reader, LengthDelimitedCodec::builder().new_codec());
    let mut sink = FramedWrite::new(writer, LengthDelimitedCodec::builder().new_codec());
    let mut name = names.get_unique();
    let item = format!("{HELP_MSG}\nYou are {name}");
    let (mut dst, i) = LengthPrefix::encode(&item);
    sink.send(dst.copy_to_bytes(i)).await?;

    // 用户房间
    let mut room_name = MAIN.to_owned();
    let mut room_tx = rooms.join(&room_name, &name);
    let mut room_rx = room_tx.subscribe();
    let _ = room_tx.send(format!("{name} 进入房间 {room_name}"));

    // sink.send(format!("{HELP_MSG}\nYou are {name}")).await?;
    let result: anyhow::Result<()> = loop {
        tokio::select! {
            user_msg = stream.next() => {
                let mut user_msg = match user_msg {
                    Some(msg) => b!(msg),
                    None => break Ok(()),
                };
                let msg = LengthPrefix::decode(&mut user_msg);
                let data = match msg {
                    Some(msg) => msg,
                    None => break Ok(()),
                };

                if data.starts_with("/help") {
                  let item = format!("{HELP_MSG}");
                  let (mut dst,i) = LengthPrefix::encode(&item);
                  sink.send(dst.copy_to_bytes(i)).await?;
                } else if data.starts_with("/name") {
                    let n = data.split_ascii_whitespace().nth(1);
                    let new_name = match n {
                        Some(n) => n,
                        None => {
                          let item = format!("输入的名字不合法");
                          let (mut dst,i) = LengthPrefix::encode(&item);
                          sink.send(dst.copy_to_bytes(i)).await?;
                          continue
                        },
                    };

                    let new_name = new_name
                        .to_owned();
                    let changed_name = names.insert(new_name.clone());
                    if changed_name {
                        b!(room_tx.send(format!("{name} is now {new_name}")));
                        rooms.change_name(&room_name, &name, &new_name);
                        name = new_name;
                    } else {
                      let item = format!("{new_name} is already taken");
                      let (mut dst,i) = LengthPrefix::encode(&item);
                      sink.send(dst.copy_to_bytes(i)).await?;
                    }
                } else if data.starts_with("/quit") {
                    break Ok(());
                }else if data.starts_with("/join") {
                  let new_room = data
                    .split_ascii_whitespace()
                    .nth(1)
                    .unwrap()
                    .to_owned();
                  if new_room == room_name {
                    let item = format!("{name} 已经存在于 {room_name} 房间");
                    let (mut dst,i) = LengthPrefix::encode(&item);
                    sink.send(dst.copy_to_bytes(i)).await?;
                    continue;
                  }
                  room_tx = rooms.change(&room_name, &new_room, &name);
                  room_rx = room_tx.subscribe();
                  room_name = new_room;
                  b!(room_tx.send(format!("{name} 离开 {room_name}")));
                } else if data.starts_with("/rooms") {
                  let  rooms_list = rooms.list();
                  let item = rooms_list
                      .into_iter()
                      .map(|(name, count)| format!("{name} ({count})"))
                      .collect::<Vec<_>>()
                      .join(",");
                  let item = format!("{item}");
                  let (mut dst,i) = LengthPrefix::encode(&item);
                  sink.send(dst.copy_to_bytes(i)).await?;
                }else if data.starts_with("/users") {
                  let users_list = rooms
                      .list_users(&room_name)
                      .unwrap()
                      .join(", ");
                  let item = format!("[Users] - [{users_list}]");
                  let (mut dst,i) = LengthPrefix::encode(&item);
                  sink.send(dst.copy_to_bytes(i)).await?;
                } else {
                    let message =  format!("[{name}][{room_name}]: {data}");
                    // println!("自己发的消息被记录:{message}");
                    b!(room_tx.send(message));
                }
            },
            peer_msg = room_rx.recv() => {
                let peer_msg = b!(peer_msg);
                let item = format!("{peer_msg}");
                println!("接受通道消息:{item}");
                let (mut dst,i) = LengthPrefix::encode(&item);
                sink.send(dst.copy_to_bytes(i)).await?;
                // b!(sink.send(peer_msg).await);
            },
        }
    };
    // 断开连接后删除资源
    rooms.leave(&room_name, &name);
    let _ = room_tx.send(format!("{name} 退出 {room_name}"));
    names.remove(&name);
    result
}
