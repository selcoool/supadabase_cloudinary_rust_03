use crate::models::user::UserImage;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
pub async fn create_user_image(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    image_url: &str,
    public_id: &str,
) -> Result<UserImage, sqlx::Error> {
    sqlx::query_as::<_, UserImage>(
        r#"
        INSERT INTO user_images (
            user_id, image_url, public_id
        )
        VALUES ($1, $2, $3)
        RETURNING id, user_id, image_url, public_id, created_at
        "#,
    )
    .bind(user_id)
    .bind(image_url)
    .bind(public_id)
    .fetch_one(&mut **tx)
    .await
}
pub async fn get_images_by_user_id(
    pool: &sqlx::PgPool,
    user_id: Uuid,
) -> Result<Vec<UserImage>, sqlx::Error> {
    sqlx::query_as::<_, UserImage>(
        r#"
        SELECT id, user_id, image_url, public_id, created_at
        FROM user_images
        WHERE user_id = $1
        ORDER BY created_at ASC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn get_public_ids_by_user_id(
    pool: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar::<_, String>("SELECT public_id FROM user_images WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await
}
