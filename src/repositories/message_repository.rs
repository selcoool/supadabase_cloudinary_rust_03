use sqlx::PgPool;
use uuid::Uuid;

use crate::models::message::Message;

// LƯU TIN NHẮN
pub async fn create_message(
    pool: &PgPool,
    sender_id: Uuid,
    receiver_id: Option<Uuid>,
    group_id: Option<i64>,
    message_type: &str,
    content: &str,
) -> Result<Message, sqlx::Error> {
    sqlx::query_as::<_, Message>(
        r#"
        INSERT INTO messages (
            sender_id,
            receiver_id,
            group_id,
            message_type,
            content
        )
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(sender_id)
    .bind(receiver_id)
    .bind(group_id)
    .bind(message_type)
    .bind(content)
    .fetch_one(pool)
    .await
}

// KIỂM TRA USER
pub async fn user_exists(pool: &PgPool, user_id: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(user_id)
        .fetch_one(pool)
        .await
}

// KIỂM TRA ADMIN
pub async fn is_admin(pool: &PgPool, user_id: Uuid) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM users
            WHERE id = $1 AND role = 'admin'
        )
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

// KIỂM TRA THÀNH VIÊN GROUP
pub async fn is_group_member(
    pool: &PgPool,
    group_id: i64,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM group_members
            WHERE group_id = $1 AND user_id = $2
        )
        "#,
    )
    .bind(group_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

// LẤY DANH SÁCH THÀNH VIÊN GROUP
pub async fn get_group_members(pool: &PgPool, group_id: i64) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM group_members WHERE group_id = $1")
        .bind(group_id)
        .fetch_all(pool)
        .await
}

// THÊM THÀNH VIÊN GROUP
pub async fn add_members(
    pool: &PgPool,
    group_id: i64,
    user_ids: &[Uuid],
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO group_members (group_id, user_id)
        SELECT $1, unnest($2::UUID[])
        ON CONFLICT (group_id, user_id) DO NOTHING
        "#,
    )
    .bind(group_id)
    .bind(user_ids)
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

// =====================================
// PRIVATE CHAT HISTORY
// =====================================

pub async fn get_private_messages(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<Vec<Message>, sqlx::Error> {
    sqlx::query_as::<_, Message>(
        r#"
        SELECT *
        FROM messages
        WHERE message_type = 'private'
        AND (
            sender_id = $1
            OR receiver_id = $1
        )
        ORDER BY created_at ASC, id ASC
        LIMIT 100
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

// =====================================
// GROUP CHAT HISTORY
// =====================================

pub async fn get_group_messages(pool: &PgPool, user_id: Uuid) -> Result<Vec<Message>, sqlx::Error> {
    sqlx::query_as::<_, Message>(
        r#"
        SELECT m.*
        FROM messages m
        INNER JOIN group_members gm
            ON gm.group_id = m.group_id
        WHERE gm.user_id = $1
        AND m.message_type = 'group'
        ORDER BY m.created_at ASC, m.id ASC
        LIMIT 100
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

// =====================================
// ALL CHAT HISTORY
// =====================================

pub async fn get_all_messages(pool: &PgPool) -> Result<Vec<Message>, sqlx::Error> {
    sqlx::query_as::<_, Message>(
        r#"
        SELECT *
        FROM messages
        WHERE message_type = 'all'
        ORDER BY created_at ASC, id ASC
        LIMIT 100
        "#,
    )
    .fetch_all(pool)
    .await
}

// use sqlx::PgPool;

// use crate::models::message::Message;

// // LƯU TIN NHẮN
// pub async fn create_message(
//     pool: &PgPool,
//     sender_id: i64,
//     receiver_id: Option<i64>,
//     group_id: Option<i64>,
//     message_type: &str,
//     content: &str,
// ) -> Result<Message, sqlx::Error> {
//     sqlx::query_as::<_, Message>(
//         r#"
//         INSERT INTO messages (
//             sender_id,
//             receiver_id,
//             group_id,
//             message_type,
//             content
//         )
//         VALUES ($1, $2, $3, $4, $5)
//         RETURNING *
//         "#,
//     )
//     .bind(sender_id)
//     .bind(receiver_id)
//     .bind(group_id)
//     .bind(message_type)
//     .bind(content)
//     .fetch_one(pool)
//     .await
// }

// // KIỂM TRA USER
// pub async fn user_exists(pool: &PgPool, user_id: i64) -> Result<bool, sqlx::Error> {
//     sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
//         .bind(user_id)
//         .fetch_one(pool)
//         .await
// }

// // KIỂM TRA ADMIN
// pub async fn is_admin(pool: &PgPool, user_id: i64) -> Result<bool, sqlx::Error> {
//     sqlx::query_scalar(
//         "SELECT EXISTS(
//             SELECT 1 FROM users
//             WHERE id = $1 AND role = 'admin'
//         )",
//     )
//     .bind(user_id)
//     .fetch_one(pool)
//     .await
// }

// // KIỂM TRA THÀNH VIÊN GROUP
// pub async fn is_group_member(
//     pool: &PgPool,
//     group_id: i64,
//     user_id: i64,
// ) -> Result<bool, sqlx::Error> {
//     sqlx::query_scalar(
//         "SELECT EXISTS(
//             SELECT 1 FROM group_members
//             WHERE group_id = $1 AND user_id = $2
//         )",
//     )
//     .bind(group_id)
//     .bind(user_id)
//     .fetch_one(pool)
//     .await
// }

// // LẤY THÀNH VIÊN GROUP
// pub async fn get_group_members(pool: &PgPool, group_id: i64) -> Result<Vec<i64>, sqlx::Error> {
//     sqlx::query_scalar("SELECT user_id FROM group_members WHERE group_id = $1")
//         .bind(group_id)
//         .fetch_all(pool)
//         .await
// }

// pub async fn add_members(
//     pool: &PgPool,
//     group_id: i64,
//     user_ids: &[i64],
// ) -> Result<u64, sqlx::Error> {
//     let result = sqlx::query(
//         r#"
//         INSERT INTO group_members (group_id, user_id)
//         SELECT $1, unnest($2::BIGINT[])
//         ON CONFLICT (group_id, user_id) DO NOTHING
//         "#,
//     )
//     .bind(group_id)
//     .bind(user_ids)
//     .execute(pool)
//     .await?;

//     Ok(result.rows_affected())
// }
