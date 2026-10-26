use crate::models::user::User;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
pub async fn create_user(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    email: &str,
    password_hash: &str,
    phone_number: Option<&str>,
    avatar_url: Option<&str>,
    avatar_public_id: Option<&str>,
) -> Result<User, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (
            name, email, password_hash, phone_number,
            avatar_url, avatar_public_id
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING
            id, name, email, phone_number, avatar_url,
            avatar_public_id, created_at, updated_at
        "#,
    )
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .bind(phone_number)
    .bind(avatar_url)
    .bind(avatar_public_id)
    .fetch_one(&mut **tx)
    .await
}
pub async fn get_user_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT
            id, name, email, phone_number, avatar_url,
            avatar_public_id, created_at, updated_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn get_all_users(pool: &sqlx::PgPool) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT id, name, email, password_hash, phone_number,
               avatar_url, avatar_public_id, created_at, updated_at
        FROM users
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn delete_user(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&mut **tx)
        .await?;

    Ok(result.rows_affected() > 0)
}

pub async fn update_user(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: uuid::Uuid,
    name: Option<&str>,
    email: Option<&str>,
    password_hash: Option<&str>,
    phone_number: Option<&str>,
    avatar_url: Option<&str>,
    avatar_public_id: Option<&str>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        UPDATE users
        SET
            name = COALESCE($2, name),
            email = COALESCE($3, email),
            password_hash = COALESCE($4, password_hash),
            phone_number = COALESCE($5, phone_number),
            avatar_url = COALESCE($6, avatar_url),
            avatar_public_id = COALESCE($7, avatar_public_id)
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .bind(phone_number)
    .bind(avatar_url)
    .bind(avatar_public_id)
    .execute(&mut **tx)
    .await?;

    Ok(result.rows_affected() > 0)
}
