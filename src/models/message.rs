use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Message {
    pub id: i64,
    pub sender_id: Uuid,
    pub receiver_id: Option<Uuid>,
    pub group_id: Option<i64>,
    pub message_type: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMessage {
    pub receiver_id: Option<Uuid>,
    pub group_id: Option<i64>,
    pub message_type: String,
    pub content: String,
}

// use chrono::{DateTime, Utc};
// use serde::{Deserialize, Serialize};
// use sqlx::FromRow;

// #[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
// pub struct Message {
//     pub id: i64,
//     pub sender_id: i64,
//     pub receiver_id: Option<i64>,
//     pub group_id: Option<i64>,
//     pub message_type: String,
//     pub content: String,
//     pub created_at: DateTime<Utc>,
// }

// #[derive(Debug, Deserialize)]
// pub struct CreateMessage {
//     pub message_type: String,
//     pub receiver_id: Option<i64>,
//     pub group_id: Option<i64>,
//     pub content: String,
// }
