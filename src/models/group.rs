use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AddGroupMembers {
    pub user_ids: Vec<Uuid>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChatGroup {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateGroup {
    pub name: String,
}
