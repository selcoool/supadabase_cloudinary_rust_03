use std::sync::Arc;

use actix_web::web;
use serde_json::to_string;
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    hub::connection_manager::Hub,
    models::message::{CreateMessage, Message},
    repositories::message_repository as repo,
};

pub struct AppState {
    pub pool: PgPool,
    pub hub: Arc<Mutex<Hub>>,
}

pub async fn send_message(
    state: &web::Data<AppState>,
    sender_id: Uuid,
    input: CreateMessage,
) -> Result<Message, String> {
    if input.content.trim().is_empty() {
        return Err("Content cannot be empty".into());
    }

    match input.message_type.as_str() {
        // PRIVATE MESSAGE
        "private" => {
            let receiver_id = input.receiver_id.ok_or("receiver_id required")?;

            if input.group_id.is_some() || sender_id == receiver_id {
                return Err("Invalid private message".into());
            }

            let exists = repo::user_exists(&state.pool, receiver_id)
                .await
                .map_err(|_| "Database error")?;

            if !exists {
                return Err("User not found".into());
            }

            let message = repo::create_message(
                &state.pool,
                sender_id,
                Some(receiver_id),
                None,
                "private",
                &input.content,
            )
            .await
            .map_err(|_| "Cannot save message")?;

            let data = to_string(&message).map_err(|_| "Serialize error")?;

            state.hub.lock().await.send_private(receiver_id, data);

            Ok(message)
        }

        // GROUP MESSAGE
        "group" => {
            let group_id = input.group_id.ok_or("group_id required")?;

            if input.receiver_id.is_some() {
                return Err("Invalid group message".into());
            }

            let is_member = repo::is_group_member(&state.pool, group_id, sender_id)
                .await
                .map_err(|_| "Database error")?;

            if !is_member {
                return Err("You are not a group member".into());
            }

            let members = repo::get_group_members(&state.pool, group_id)
                .await
                .map_err(|_| "Database error")?;

            let message = repo::create_message(
                &state.pool,
                sender_id,
                None,
                Some(group_id),
                "group",
                &input.content,
            )
            .await
            .map_err(|_| "Cannot save message")?;

            let data = to_string(&message).map_err(|_| "Serialize error")?;

            state.hub.lock().await.send_group(&members, data);

            Ok(message)
        }

        // BROADCAST MESSAGE
        "all" => {
            if input.receiver_id.is_some() || input.group_id.is_some() {
                return Err("Invalid broadcast message".into());
            }

            let admin = repo::is_admin(&state.pool, sender_id)
                .await
                .map_err(|_| "Database error")?;

            if !admin {
                return Err("Only admin can broadcast".into());
            }

            let message =
                repo::create_message(&state.pool, sender_id, None, None, "all", &input.content)
                    .await
                    .map_err(|_| "Cannot save message")?;

            let data = to_string(&message).map_err(|_| "Serialize error")?;

            state.hub.lock().await.send_all(data);

            Ok(message)
        }

        _ => Err("Invalid message type".into()),
    }
}

// ADD GROUP MEMBERS
// pub async fn add_group_members(
//     pool: &PgPool,
//     group_id: i64,
//     user_ids: Vec<Uuid>,
// ) -> Result<u64, String> {
//     if user_ids.is_empty() {
//         return Err("Danh sách user_ids không được rỗng".to_string());
//     }

//     if user_ids.len() > 100 {
//         return Err("Chỉ được thêm tối đa 100 người mỗi lần".to_string());
//     }

//     // Kiểm tra group có tồn tại không
//     let group_exists =
//         sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM chat_groups WHERE id = $1)")
//             .bind(group_id)
//             .fetch_one(pool)
//             .await
//             .map_err(|e| e.to_string())?;

//     if !group_exists {
//         return Err("Group không tồn tại".to_string());
//     }

//     // Loại bỏ UUID trùng lặp
//     let unique_user_ids: Vec<Uuid> = user_ids
//         .iter()
//         .copied()
//         .collect::<std::collections::HashSet<_>>()
//         .into_iter()
//         .collect();

//     // Kiểm tra tất cả user có tồn tại không
//     let existing_count = sqlx::query_scalar::<_, i64>(
//         r#"
//         SELECT COUNT(DISTINCT id)
//         FROM users
//         WHERE id = ANY($1::UUID[])
//         "#,
//     )
//     .bind(&unique_user_ids)
//     .fetch_one(pool)
//     .await
//     .map_err(|e| e.to_string())?;

//     if existing_count != unique_user_ids.len() as i64 {
//         return Err("Có user_id không tồn tại".to_string());
//     }

//     // Thêm thành viên
//     let added = repo::add_members(pool, group_id, &unique_user_ids)
//         .await
//         .map_err(|e| e.to_string())?;

//     Ok(added)
// }

// use std::sync::Arc;

// use actix_web::web;
// use serde_json::to_string;
// use sqlx::PgPool;
// use tokio::sync::Mutex;

// use crate::{
//     hub::connection_manager::Hub,
//     models::message::{CreateMessage, Message},
//     repositories::message_repository as repo,
// };

// pub struct AppState {
//     pub pool: PgPool,
//     pub hub: Arc<Mutex<Hub>>,
// }

// pub async fn send_message(
//     state: &web::Data<AppState>,
//     sender_id: i64,
//     input: CreateMessage,
// ) -> Result<Message, String> {
//     if input.content.trim().is_empty() {
//         return Err("Content cannot be empty".into());
//     }

//     match input.message_type.as_str() {
//         // PRIVATE
//         "private" => {
//             let receiver_id = input.receiver_id.ok_or("receiver_id required")?;

//             if input.group_id.is_some() || sender_id == receiver_id {
//                 return Err("Invalid private message".into());
//             }

//             let exists = repo::user_exists(&state.pool, receiver_id)
//                 .await
//                 .map_err(|_| "Database error")?;

//             if !exists {
//                 return Err("User not found".into());
//             }

//             let message = repo::create_message(
//                 &state.pool,
//                 sender_id,
//                 Some(receiver_id),
//                 None,
//                 "private",
//                 &input.content,
//             )
//             .await
//             .map_err(|_| "Cannot save message")?;

//             let data = to_string(&message).map_err(|_| "Serialize error")?;

//             state.hub.lock().await.send_private(receiver_id, data);

//             Ok(message)
//         }

//         // GROUP
//         "group" => {
//             let group_id = input.group_id.ok_or("group_id required")?;

//             if input.receiver_id.is_some() {
//                 return Err("Invalid group message".into());
//             }

//             let is_member = repo::is_group_member(&state.pool, group_id, sender_id)
//                 .await
//                 .map_err(|_| "Database error")?;

//             if !is_member {
//                 return Err("You are not a group member".into());
//             }

//             let members = repo::get_group_members(&state.pool, group_id)
//                 .await
//                 .map_err(|_| "Database error")?;

//             let message = repo::create_message(
//                 &state.pool,
//                 sender_id,
//                 None,
//                 Some(group_id),
//                 "group",
//                 &input.content,
//             )
//             .await
//             .map_err(|_| "Cannot save message")?;

//             let data = to_string(&message).map_err(|_| "Serialize error")?;

//             state.hub.lock().await.send_group(&members, data);

//             Ok(message)
//         }

//         // ALL
//         "all" => {
//             if input.receiver_id.is_some() || input.group_id.is_some() {
//                 return Err("Invalid broadcast message".into());
//             }

//             let admin = repo::is_admin(&state.pool, sender_id)
//                 .await
//                 .map_err(|_| "Database error")?;

//             if !admin {
//                 return Err("Only admin can broadcast".into());
//             }

//             let message =
//                 repo::create_message(&state.pool, sender_id, None, None, "all", &input.content)
//                     .await
//                     .map_err(|_| "Cannot save message")?;

//             let data = to_string(&message).map_err(|_| "Serialize error")?;

//             state.hub.lock().await.send_all(data);

//             Ok(message)
//         }

//         _ => Err("Invalid message type".into()),
//     }
// }

// pub async fn add_group_members(
//     pool: &PgPool,
//     group_id: i64,
//     user_ids: Vec<i64>,
// ) -> Result<u64, String> {
//     if user_ids.is_empty() {
//         return Err("Danh sách user_ids không được rỗng".to_string());
//     }

//     if user_ids.iter().any(|id| *id <= 0) {
//         return Err("user_id phải lớn hơn 0".to_string());
//     }

//     if user_ids.len() > 100 {
//         return Err("Chỉ được thêm tối đa 100 người mỗi lần".to_string());
//     }

//     // Kiểm tra group
//     let group_exists =
//         sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM chat_groups WHERE id = $1)")
//             .bind(group_id)
//             .fetch_one(pool)
//             .await
//             .map_err(|e| e.to_string())?;

//     if !group_exists {
//         return Err("Group không tồn tại".to_string());
//     }

//     // Kiểm tra tất cả user có tồn tại không
//     let existing_count = sqlx::query_scalar::<_, i64>(
//         r#"
//         SELECT COUNT(DISTINCT id)
//         FROM users
//         WHERE id = ANY($1::BIGINT[])
//         "#,
//     )
//     .bind(&user_ids)
//     .fetch_one(pool)
//     .await
//     .map_err(|e| e.to_string())?;

//     let unique_count = user_ids
//         .iter()
//         .collect::<std::collections::HashSet<_>>()
//         .len();

//     if existing_count != unique_count as i64 {
//         return Err("Có user_id không tồn tại".to_string());
//     }

//     // Thêm thành viên
//     let added = repo::add_members(pool, group_id, &user_ids)
//         .await
//         .map_err(|e| e.to_string())?;

//     Ok(added)
// }
