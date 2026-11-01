use sqlx::PgPool;
use uuid::Uuid;

use crate::models::group::ChatGroup;

// =====================================
// ADD GROUP MEMBERS
// =====================================

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
// CREATE GROUP
// =====================================

pub async fn create_group(pool: &PgPool, name: &str) -> Result<ChatGroup, sqlx::Error> {
    sqlx::query_as::<_, ChatGroup>(
        r#"
        INSERT INTO chat_groups (name)
        VALUES ($1)
        RETURNING id, name
        "#,
    )
    .bind(name)
    .fetch_one(pool)
    .await
}

// use sqlx::PgPool;

// use crate::models::group::ChatGroup;

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

// pub async fn create_group(pool: &PgPool, name: &str) -> Result<ChatGroup, sqlx::Error> {
//     sqlx::query_as::<_, ChatGroup>(
//         r#"
//         INSERT INTO chat_groups (name)
//         VALUES ($1)
//         RETURNING id, name
//         "#,
//     )
//     .bind(name)
//     .fetch_one(pool)
//     .await
// }
