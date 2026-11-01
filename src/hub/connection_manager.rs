use std::collections::HashMap;

use tokio::sync::mpsc;
use uuid::Uuid;

pub struct Hub {
    pub users: HashMap<Uuid, HashMap<Uuid, mpsc::UnboundedSender<String>>>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
        }
    }

    // Đăng ký kết nối
    pub fn connect(&mut self, user_id: Uuid, tx: mpsc::UnboundedSender<String>) -> Uuid {
        let connection_id = Uuid::new_v4();

        self.users
            .entry(user_id)
            .or_default()
            .insert(connection_id, tx);

        connection_id
    }

    // Ngắt kết nối
    pub fn disconnect(&mut self, user_id: Uuid, connection_id: Uuid) {
        if let Some(connections) = self.users.get_mut(&user_id) {
            connections.remove(&connection_id);

            if connections.is_empty() {
                self.users.remove(&user_id);
            }
        }
    }

    // Gửi tin nhắn riêng
    pub fn send_private(&self, receiver_id: Uuid, message: String) {
        if let Some(connections) = self.users.get(&receiver_id) {
            for tx in connections.values() {
                let _ = tx.send(message.clone());
            }
        }
    }

    // Gửi tin nhắn group
    pub fn send_group(&self, members: &[Uuid], message: String) {
        for user_id in members {
            self.send_private(*user_id, message.clone());
        }
    }

    // Gửi tin nhắn toàn bộ user
    pub fn send_all(&self, message: String) {
        for connections in self.users.values() {
            for tx in connections.values() {
                let _ = tx.send(message.clone());
            }
        }
    }
}

// use std::collections::HashMap;

// use tokio::sync::mpsc;
// use uuid::Uuid;

// pub struct Hub {
//     pub users: HashMap<i64, HashMap<Uuid, mpsc::UnboundedSender<String>>>,
// }

// impl Hub {
//     pub fn new() -> Self {
//         Self {
//             users: HashMap::new(),
//         }
//     }

//     // CONNECT
//     pub fn connect(&mut self, user_id: i64, sender: mpsc::UnboundedSender<String>) -> Uuid {
//         let id = Uuid::new_v4();

//         self.users.entry(user_id).or_default().insert(id, sender);

//         id
//     }

//     // DISCONNECT
//     pub fn disconnect(&mut self, user_id: i64, connection_id: Uuid) {
//         if let Some(connections) = self.users.get_mut(&user_id) {
//             connections.remove(&connection_id);

//             if connections.is_empty() {
//                 self.users.remove(&user_id);
//             }
//         }
//     }

//     // PRIVATE
//     pub fn send_private(&self, receiver_id: i64, message: String) {
//         if let Some(connections) = self.users.get(&receiver_id) {
//             for sender in connections.values() {
//                 let _ = sender.send(message.clone());
//             }
//         }
//     }

//     // GROUP
//     pub fn send_group(&self, members: &[i64], message: String) {
//         for user_id in members {
//             if let Some(connections) = self.users.get(user_id) {
//                 for sender in connections.values() {
//                     let _ = sender.send(message.clone());
//                 }
//             }
//         }
//     }

//     // ALL
//     pub fn send_all(&self, message: String) {
//         for connections in self.users.values() {
//             for sender in connections.values() {
//                 let _ = sender.send(message.clone());
//             }
//         }
//     }
// }
