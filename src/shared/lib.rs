mod adjectives;
mod animals;

use std::collections::{HashSet,HashMap};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::broadcast;
use tokio::sync::broadcast::{Sender, Receiver};
use adjectives::ADJECTIVES;
use animals::ANIMALS;

pub fn random_name() -> String {
    // 生成随机的名字
    let adjective = fastrand::choice(ADJECTIVES).unwrap();
    let animal = fastrand::choice(ANIMALS).unwrap();
    format!("{adjective}{animal}")
}

macro_rules! b {
    ($result:expr) => {
        match $result {
            Ok(ok) => ok,
            Err(err) => break Err(err.into()),
        }
    }
}
pub(crate) use b;

pub(crate) mod length_code;

// Names 是一个线程安全的集合，用于管理用户的昵称

#[derive(Clone)]
pub struct Names(Arc<Mutex<HashSet<String>>>);

impl Names {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(HashSet::new())))
    }
    pub fn insert(&self, name: String) -> bool {
        self.0.lock().unwrap().insert(name)
    }
    pub fn remove(&self, name: &str) -> bool {
        self.0.lock().unwrap().remove(name)
    }
    pub fn get_unique(&self) -> String {
        let mut name = random_name();
        let mut guard = self.0.lock().unwrap();
        while !guard.insert(name.clone()) {
            name = random_name();
        }
        name
    }
}

pub const MAIN: &str = "MAIN";

struct Room {
    tx: Sender<String>,
    // 当前房间的用户列表
    users: HashSet<String>,
}

impl Room {
    fn new() -> Self {
        let (tx, _) = broadcast::channel::<String>(32);
        let users = HashSet::new();
        Self { tx, users }
    }
}

#[derive(Clone)]
pub struct Rooms(Arc<RwLock<HashMap<String, Room>>>);

impl Rooms {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(HashMap::new())))
    }
    // 进入房间
    pub fn join(&self, room_name: &str, user_name: &str) -> Sender<String> {
        let mut write_guard = self.0.write().unwrap();
        let room = write_guard
            .entry(room_name.to_owned())
            .or_insert(Room::new());

        room.users.insert(user_name.to_owned());
        room.tx.clone()
    }
    // 退出房间
    pub fn leave(&self, room_name: &str, user_name: &str) {
        let mut write_guard = self.0.write().unwrap();
        let mut delete_room = false;
        if let Some(room) = write_guard.get_mut(room_name) {
            room.users.remove(user_name);
            delete_room = room.tx.receiver_count() <= 1;
        }

        if delete_room {
            write_guard.remove(room_name);
        }
    }
    // 改变房间
    pub fn change(&self, prev_room: &str, next_room: &str, user_name: &str) -> Sender<String> {
        // 退出当前房间
        self.leave(prev_room, user_name);
        // 进入其他房间
        self.join(next_room, user_name)
    }
    // 修改名称
    pub fn change_name(&self, room_name: &str, prev_name: &str, new_name: &str) {
        let mut write_guard = self.0.write().unwrap();
        if let Some(room) = write_guard.get_mut(room_name) {
            room.users.remove(prev_name);
            room.users.insert(new_name.to_owned());
        }
    }
    // 列出房间
    pub fn list(&self) -> Vec<(String, usize)> {
        let mut list: Vec<_> = self
            .0
            .read()
            .unwrap()
            .iter()
            .map(|(name, room)| (name.to_owned(), room.tx.receiver_count()))
            .collect();
        list.sort_by(|a, b| {
            use std::cmp::Ordering::*;
            match b.1.cmp(&a.1) {
                Equal => a.0.cmp(&b.0),
                ordering => ordering,
            }
        });
        list
    }
    // 列出房间内的用户
    pub fn list_users(&self, room_name: &str) -> Option<Vec<String>> {
        self.0.read().unwrap().get(room_name).map(|room| {
            let mut users = room.users.iter().cloned().collect::<Vec<_>>();
            users.sort();
            users
        })
    }
}