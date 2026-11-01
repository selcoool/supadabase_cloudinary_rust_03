use crate::models::user::UserResponse;
use crate::repositories::{user_image_repository, user_repository};
use crate::services::cloudinary_service::{CloudinaryService, UploadedImage};
use sqlx::PgPool;
pub async fn create_user(
    pool: &PgPool,
    name: &str,
    email: &str,
    password_hash: &str,
    phone_number: Option<&str>,
    avatar: Option<&UploadedImage>,
    related_images: &[UploadedImage],
) -> Result<UserResponse, String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let user = user_repository::create_user(
        &mut tx,
        name,
        email,
        password_hash,
        phone_number,
        avatar.map(|image| image.url.as_str()),
        avatar.map(|image| image.public_id.as_str()),
    )
    .await
    .map_err(|e| e.to_string())?;
    for image in related_images {
        user_image_repository::create_user_image(&mut tx, user.id, &image.url, &image.public_id)
            .await
            .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())?;
    let images = user_image_repository::get_images_by_user_id(pool, user.id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(UserResponse::new(user, images))
}
pub async fn get_user(pool: &PgPool, id: uuid::Uuid) -> Result<Option<UserResponse>, String> {
    let Some(user) = user_repository::get_user_by_id(pool, id)
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    let images = user_image_repository::get_images_by_user_id(pool, id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Some(UserResponse::new(user, images)))
}

pub async fn get_all_users(pool: &PgPool) -> Result<Vec<UserResponse>, String> {
    let users = user_repository::get_all_users(pool)
        .await
        .map_err(|e| e.to_string())?;
    // dbg!(&users);
    let mut responses = Vec::with_capacity(users.len());
    // dbg!(&users.len());

    for user in users {
        let images = user_image_repository::get_images_by_user_id(pool, user.id)
            .await
            .map_err(|e| e.to_string())?;

        responses.push(UserResponse::new(user, images));
    }

    Ok(responses)
}

pub async fn delete_user(
    pool: &PgPool,
    cloudinary: &crate::services::cloudinary_service::CloudinaryService,
    user_id: uuid::Uuid,
) -> Result<bool, String> {
    // Kiểm tra user và lấy avatar public_id.
    let avatar_public_id: Option<String> =
        sqlx::query_scalar("SELECT avatar_public_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?
            .flatten();

    // Nếu user không tồn tại, trả về false.
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    if !exists {
        return Ok(false);
    }

    // Lấy public_id của tất cả ảnh liên quan trước khi xóa.
    let related_ids = user_image_repository::get_public_ids_by_user_id(pool, user_id)
        .await
        .map_err(|e| e.to_string())?;

    // Xóa dữ liệu database trong một transaction.
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM user_images WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    // Gọi repository xóa user.
    let deleted = user_repository::delete_user(&mut tx, user_id)
        .await
        .map_err(|e| e.to_string())?;

    if !deleted {
        return Ok(false);
    }

    tx.commit().await.map_err(|e| e.to_string())?;

    // Xóa avatar nếu user có avatar.
    if let Some(public_id) = avatar_public_id {
        if let Err(error) = cloudinary.destroy(&public_id).await {
            eprintln!("Avatar cleanup failed for user {user_id}: {error}");
        }
    }

    // Xóa tất cả related images.
    for public_id in related_ids {
        if let Err(error) = cloudinary.destroy(&public_id).await {
            eprintln!("Related image cleanup failed for user {user_id}: {error}");
        }
    }

    Ok(true)
}

pub async fn update_user(
    pool: &PgPool,
    cloudinary: &CloudinaryService,
    user_id: uuid::Uuid,
    name: Option<&str>,
    email: Option<&str>,
    password_hash: Option<&str>,
    phone_number: Option<&str>,
    avatar: Option<&UploadedImage>,
    related_images: &[UploadedImage],
    replace_related_images: bool,
) -> Result<bool, String> {
    // Kiểm tra user và lấy avatar cũ.
    let old_avatar: Option<Option<String>> =
        sqlx::query_scalar("SELECT avatar_public_id FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| e.to_string())?;

    let Some(old_avatar_public_id) = old_avatar else {
        return Ok(false);
    };

    // Lấy public_id ảnh liên quan cũ trước khi thay thế.
    let old_related_ids = if replace_related_images {
        user_image_repository::get_public_ids_by_user_id(pool, user_id)
            .await
            .map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };

    // Bắt đầu transaction.
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    let updated = user_repository::update_user(
        &mut tx,
        user_id,
        name,
        email,
        password_hash,
        phone_number,
        avatar.map(|image| image.url.as_str()),
        avatar.map(|image| image.public_id.as_str()),
    )
    .await
    .map_err(|e| e.to_string())?;

    if !updated {
        return Ok(false);
    }

    // Nếu client gửi related_images hoặc yêu cầu xóa hết,
    // xóa các bản ghi ảnh liên quan cũ trong transaction.
    if replace_related_images {
        sqlx::query("DELETE FROM user_images WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;

        // Thêm danh sách ảnh mới. Nếu danh sách rỗng,
        // user sẽ không còn ảnh liên quan nào.
        for image in related_images {
            user_image_repository::create_user_image(
                &mut tx,
                user_id,
                &image.url,
                &image.public_id,
            )
            .await
            .map_err(|e| e.to_string())?;
        }
    }

    // Chỉ tiếp tục xóa ảnh cũ sau khi database commit thành công.
    tx.commit().await.map_err(|e| e.to_string())?;

    // Xóa avatar cũ nếu đã thay avatar.
    if let (Some(old_id), Some(new_avatar)) = (old_avatar_public_id, avatar) {
        if old_id != new_avatar.public_id {
            if let Err(error) = cloudinary.destroy(&old_id).await {
                eprintln!("Old avatar cleanup failed for user {user_id}: {error}");
            }
        }
    }

    // Xóa các related images cũ trên Cloudinary.
    // Không xóa public_id còn được sử dụng trong danh sách mới.
    for old_id in old_related_ids {
        let still_used = related_images.iter().any(|image| image.public_id == old_id);

        if !still_used {
            if let Err(error) = cloudinary.destroy(&old_id).await {
                eprintln!("Old related image cleanup failed for user {user_id}: {error}");
            }
        }
    }

    Ok(true)
}

// pub async fn update_user(
//     pool: &PgPool,
//     cloudinary: &CloudinaryService,
//     user_id: uuid::Uuid,
//     name: Option<&str>,
//     email: Option<&str>,
//     password_hash: Option<&str>,
//     phone_number: Option<&str>,
//     avatar: Option<&UploadedImage>,
//     related_images: &[UploadedImage],
// ) -> Result<bool, String> {
//     // Lấy avatar cũ. None bên ngoài nghĩa là user không tồn tại;
//     // None bên trong nghĩa là user tồn tại nhưng chưa có avatar.
//     let old_avatar: Option<Option<String>> =
//         sqlx::query_scalar("SELECT avatar_public_id FROM users WHERE id = $1")
//             .bind(user_id)
//             .fetch_optional(pool)
//             .await
//             .map_err(|e| e.to_string())?;

//     let Some(old_avatar_public_id) = old_avatar else {
//         return Ok(false);
//     };

//     let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

//     let updated = user_repository::update_user(
//         &mut tx,
//         user_id,
//         name,
//         email,
//         password_hash,
//         phone_number,
//         avatar.map(|image| image.url.as_str()),
//         avatar.map(|image| image.public_id.as_str()),
//     )
//     .await
//     .map_err(|e| e.to_string())?;

//     if !updated {
//         return Ok(false);
//     }

//     // Thêm ảnh liên quan mới, không xóa ảnh hiện có.
//     for image in related_images {
//         user_image_repository::create_user_image(&mut tx, user_id, &image.url, &image.public_id)
//             .await
//             .map_err(|e| e.to_string())?;
//     }

//     // Nếu commit thất bại, transaction sẽ rollback.
//     tx.commit().await.map_err(|e| e.to_string())?;

//     // Chỉ xóa avatar cũ sau khi database đã cập nhật thành công.
//     if let (Some(old_id), Some(new_avatar)) = (old_avatar_public_id, avatar) {
//         if old_id != new_avatar.public_id {
//             if let Err(error) = cloudinary.destroy(&old_id).await {
//                 eprintln!("Old avatar cleanup failed for user {user_id}: {error}");
//             }
//         }
//     }

//     Ok(true)
// }
