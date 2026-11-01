// Bạn cần thêm trường role vào cả User và UserResponse, đồng thời cập nhật hàm UserResponse::new().

// Đây là code đầy đủ đã cập nhật:
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub phone_number: Option<String>,
    pub avatar_url: Option<String>,
    pub avatar_public_id: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct UserImage {
    pub id: Uuid,
    pub user_id: Uuid,
    pub image_url: String,
    pub public_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub phone_number: Option<String>,
    pub avatar: Option<String>,
    pub related_images: Vec<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserResponse {
    pub fn new(user: User, images: Vec<UserImage>) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            phone_number: user.phone_number,
            avatar: user.avatar_url,
            related_images: images.into_iter().map(|image| image.image_url).collect(),
            role: user.role,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

// use chrono::{DateTime, Utc};
// use serde::Serialize;
// use uuid::Uuid;
// #[derive(Debug, Serialize, sqlx::FromRow)]
// pub struct User {
//     pub id: Uuid,
//     pub name: String,
//     pub email: String,
//     pub phone_number: Option<String>,
//     pub avatar_url: Option<String>,
//     pub avatar_public_id: Option<String>,
//     pub created_at: DateTime<Utc>,
//     pub updated_at: DateTime<Utc>,
// }
// #[derive(Debug, Serialize, sqlx::FromRow)]
// pub struct UserImage {
//     pub id: Uuid,
//     pub user_id: Uuid,
//     pub image_url: String,
//     pub public_id: String,
//     pub created_at: DateTime<Utc>,
// }
// #[derive(Debug, Serialize)]
// pub struct UserResponse {
//     pub id: Uuid,
//     pub name: String,
//     pub email: String,
//     pub phone_number: Option<String>,
//     pub avatar: Option<String>,
//     pub related_images: Vec<String>,
//     pub created_at: DateTime<Utc>,
//     pub updated_at: DateTime<Utc>,
// }
// impl UserResponse {
//     pub fn new(user: User, images: Vec<UserImage>) -> Self {
//         Self {
//             id: user.id,
//             name: user.name,
//             email: user.email,
//             phone_number: user.phone_number,
//             avatar: user.avatar_url,
//             related_images: images.into_iter().map(|image| image.image_url).collect(),
//             created_at: user.created_at,
//             updated_at: user.updated_at,
//         }
//     }
// }
