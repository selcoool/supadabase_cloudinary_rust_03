use crate::services::{
    cloudinary_service::{CloudinaryService, UploadedImage},
    user_service,
};
use actix_multipart::{Field, Multipart};
use actix_web::{HttpResponse, Responder, web};
use futures_util::StreamExt;
use serde_json::json;
use sqlx::PgPool;
use std::collections::HashMap;
use uuid::Uuid;

use crate::utils::upload_helpers::{
    MAX_RELATED_IMAGES, PendingImage, cleanup_images, read_image_field, read_text_field,
};

pub async fn create_user(
    pool: web::Data<PgPool>,
    cloudinary: web::Data<CloudinaryService>,
    mut payload: Multipart,
) -> impl Responder {
    let mut text_fields: HashMap<String, String> = HashMap::new();
    let mut avatar_file: Option<PendingImage> = None;
    let mut related_files: Vec<PendingImage> = Vec::new();

    // Đọc toàn bộ multipart trước khi upload ảnh.
    while let Some(item) = payload.next().await {
        let field = match item {
            Ok(field) => field,
            Err(error) => {
                eprintln!("Multipart error: {error}");

                return HttpResponse::BadRequest().json(json!({
                    "message": "Invalid multipart data"
                }));
            }
        };

        let field_name = match field.content_disposition().and_then(|cd| cd.get_name()) {
            Some(name) => name.to_string(),
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "message": "Multipart field is missing a name"
                }));
            }
        };

        match field_name.as_str() {
            "name" | "email" | "password" | "phone_number" => {
                let value = match read_text_field(field).await {
                    Ok(value) => value,
                    Err(error) => {
                        return HttpResponse::BadRequest().json(json!({
                            "message": error
                        }));
                    }
                };

                text_fields.insert(field_name, value);
            }

            "avatar" => {
                if avatar_file.is_some() {
                    return HttpResponse::BadRequest().json(json!({
                        "message": "Only one avatar is allowed"
                    }));
                }

                avatar_file = match read_image_field(field).await {
                    Ok(file) => Some(file),
                    Err(error) => {
                        return HttpResponse::BadRequest().json(json!({
                            "message": error
                        }));
                    }
                };
            }

            "related_images" => {
                if related_files.len() >= MAX_RELATED_IMAGES {
                    return HttpResponse::BadRequest().json(json!({
                        "message": "Maximum 20 related images are allowed"
                    }));
                }

                match read_image_field(field).await {
                    Ok(file) => related_files.push(file),
                    Err(error) => {
                        return HttpResponse::BadRequest().json(json!({
                            "message": error
                        }));
                    }
                }
            }

            _ => {
                return HttpResponse::BadRequest().json(json!({
                    "message": "Unknown multipart field",
                    "field": field_name
                }));
            }
        }
    }

    // Lấy dữ liệu text.
    let name = text_fields.get("name").map(String::as_str).unwrap_or("");

    let email = text_fields.get("email").map(String::as_str).unwrap_or("");

    let password = text_fields
        .get("password")
        .map(String::as_str)
        .unwrap_or("");

    let phone_number = text_fields
        .get("phone_number")
        .filter(|value| !value.is_empty())
        .map(String::as_str);

    // Kiểm tra dữ liệu bắt buộc.
    if name.is_empty() || email.is_empty() || password.is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "message": "name, email and password are required"
        }));
    }

    if password.len() < 8 {
        return HttpResponse::BadRequest().json(json!({
            "message": "password must contain at least 8 bytes"
        }));
    }

    if email.len() > 254 || name.len() > 200 {
        return HttpResponse::BadRequest().json(json!({
            "message": "name or email is too long"
        }));
    }

    // Hash password.
    let password_hash = match bcrypt::hash(password, bcrypt::DEFAULT_COST) {
        Ok(hash) => hash,
        Err(error) => {
            eprintln!("Password hashing failed: {error}");

            return HttpResponse::InternalServerError().json(json!({
                "message": "Cannot process password"
            }));
        }
    };

    // let cloudinary = &state.cloudinary;

    let mut uploaded_avatar: Option<UploadedImage> = None;
    let mut uploaded_related: Vec<UploadedImage> = Vec::new();

    // Upload avatar vào CLOUDINARY_AVATAR_FOLDER.
    if let Some(file) = avatar_file {
        match cloudinary.upload_avatar(file.bytes, &file.filename).await {
            Ok(image) => {
                uploaded_avatar = Some(image);
            }

            Err(error) => {
                eprintln!("Avatar upload failed: {error}");

                return HttpResponse::BadRequest().json(json!({
                    "message": "Cannot upload avatar"
                }));
            }
        }
    }

    // Upload related images vào CLOUDINARY_RELATED_IMAGES_FOLDER.
    for file in related_files {
        match cloudinary
            .upload_related_image(file.bytes, &file.filename)
            .await
        {
            Ok(image) => {
                uploaded_related.push(image);
            }

            Err(error) => {
                eprintln!("Related image upload failed: {error}");

                // cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;

                cleanup_images(cloudinary.get_ref(), &uploaded_avatar, &uploaded_related).await;

                return HttpResponse::BadRequest().json(json!({
                    "message": "Cannot upload related image"
                }));
            }
        }
    }

    // Tạo user và lưu thông tin ảnh vào database.
    match user_service::create_user(
        pool.get_ref(),
        name,
        email,
        &password_hash,
        phone_number,
        uploaded_avatar.as_ref(),
        &uploaded_related,
    )
    .await
    {
        Ok(user) => HttpResponse::Created().json(json!({
            "message": "User created successfully",
            "user": user
        })),

        Err(error) => {
            eprintln!("Create user failed: {error}");

            cleanup_images(cloudinary.get_ref(), &uploaded_avatar, &uploaded_related).await;

            HttpResponse::InternalServerError().json(json!({
                "message": "Cannot create user. Check that the email is unique and the database is available."
            }))
        }
    }
}

pub async fn get_user(pool: web::Data<PgPool>, path: web::Path<String>) -> impl Responder {
    let id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "message": "Invalid user UUID"
            }));
        }
    };

    match user_service::get_user(pool.get_ref(), id).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),

        Ok(None) => HttpResponse::NotFound().json(json!({
            "message": "User not found"
        })),

        Err(error) => {
            eprintln!("Get user failed: {error}");

            HttpResponse::InternalServerError().json(json!({
                "message": "Cannot get user"
            }))
        }
    }
}

pub async fn get_all_users(pool: web::Data<PgPool>) -> impl Responder {
    match user_service::get_all_users(pool.get_ref()).await {
        Ok(users) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Users retrieved successfully",
            "count": users.len(),
            "users": users
        })),

        Err(error) => {
            eprintln!("Get all users failed: {error}");

            HttpResponse::InternalServerError().json(json!({
                "message": "Cannot get users"
            }))
        }
    }
}

pub async fn delete_user(
    pool: web::Data<PgPool>,
    cloudinary: web::Data<CloudinaryService>,
    path: web::Path<String>,
) -> impl Responder {
    let user_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "message": "Invalid user UUID"
            }));
        }
    };

    match user_service::delete_user(pool.get_ref(), cloudinary.get_ref(), user_id).await {
        Ok(true) => HttpResponse::Ok().json(json!({
            "message": "User and associated images deleted successfully"
        })),

        Ok(false) => HttpResponse::NotFound().json(json!({
            "message": "User not found"
        })),

        Err(error) => {
            eprintln!("Delete user failed: {error}");

            HttpResponse::InternalServerError().json(json!({
                "message": "Cannot delete user"
            }))
        }
    }
}

pub async fn update_user(
    pool: web::Data<PgPool>,
    cloudinary: web::Data<CloudinaryService>,
    path: web::Path<String>,
    mut payload: Multipart,
) -> impl Responder {
    let user_id = match Uuid::parse_str(&path.into_inner()) {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "message": "Invalid user UUID"
            }));
        }
    };

    let mut text_fields: HashMap<String, String> = HashMap::new();
    let mut avatar_file: Option<PendingImage> = None;
    let mut related_files: Vec<PendingImage> = Vec::new();
    let mut related_images_field_present = false;

    // Đọc toàn bộ multipart trước khi upload.
    while let Some(item) = payload.next().await {
        let field = match item {
            Ok(field) => field,
            Err(error) => {
                eprintln!("Multipart error: {error}");

                return HttpResponse::BadRequest().json(json!({
                    "message": "Invalid multipart data"
                }));
            }
        };

        let field_name = match field.content_disposition().and_then(|cd| cd.get_name()) {
            Some(name) => name.to_string(),
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "message": "Multipart field is missing a name"
                }));
            }
        };

        match field_name.as_str() {
            "name" | "email" | "password" | "phone_number" => {
                if text_fields.contains_key(&field_name) {
                    return HttpResponse::BadRequest().json(json!({
                        "message": "Duplicate text field",
                        "field": field_name
                    }));
                }

                let value = match read_text_field(field).await {
                    Ok(value) => value,
                    Err(error) => {
                        return HttpResponse::BadRequest().json(json!({
                            "message": error
                        }));
                    }
                };

                text_fields.insert(field_name, value);
            }

            "avatar" => {
                if avatar_file.is_some() {
                    return HttpResponse::BadRequest().json(json!({
                        "message": "Only one avatar is allowed"
                    }));
                }

                avatar_file = match read_image_field(field).await {
                    Ok(file) => Some(file),
                    Err(error) => {
                        return HttpResponse::BadRequest().json(json!({
                            "message": error
                        }));
                    }
                };
            }

            "related_images" => {
                related_images_field_present = true;

                if related_files.len() >= MAX_RELATED_IMAGES {
                    return HttpResponse::BadRequest().json(json!({
                        "message": "Maximum 20 related images are allowed"
                    }));
                }

                match read_image_field(field).await {
                    Ok(file) => related_files.push(file),
                    Err(error) => {
                        return HttpResponse::BadRequest().json(json!({
                            "message": error
                        }));
                    }
                }
            }

            _ => {
                return HttpResponse::BadRequest().json(json!({
                    "message": "Unknown multipart field",
                    "field": field_name
                }));
            }
        }
    }

    let name = text_fields.get("name").map(String::as_str);
    let email = text_fields.get("email").map(String::as_str);
    let password = text_fields.get("password").map(String::as_str);

    let phone_number = text_fields
        .get("phone_number")
        .filter(|value| !value.is_empty())
        .map(String::as_str);

    // Kiểm tra dữ liệu.
    if name.is_some_and(|value| value.is_empty()) || email.is_some_and(|value| value.is_empty()) {
        return HttpResponse::BadRequest().json(json!({
            "message": "name and email cannot be empty"
        }));
    }

    if password.is_some_and(|value| value.len() < 8) {
        return HttpResponse::BadRequest().json(json!({
            "message": "password must contain at least 8 bytes"
        }));
    }

    if name.is_some_and(|value| value.len() > 200) || email.is_some_and(|value| value.len() > 254) {
        return HttpResponse::BadRequest().json(json!({
            "message": "name or email is too long"
        }));
    }

    // Hash password mới nếu có gửi.
    let password_hash = match password {
        Some(value) => match bcrypt::hash(value, bcrypt::DEFAULT_COST) {
            Ok(hash) => Some(hash),
            Err(error) => {
                eprintln!("Password hashing failed: {error}");

                return HttpResponse::InternalServerError().json(json!({
                    "message": "Cannot process password"
                }));
            }
        },
        None => None,
    };

    // let cloudinary = &state.cloudinary;

    let mut uploaded_avatar: Option<UploadedImage> = None;
    let mut uploaded_related: Vec<UploadedImage> = Vec::new();

    // Upload avatar mới.
    if let Some(file) = avatar_file {
        match cloudinary.upload_avatar(file.bytes, &file.filename).await {
            Ok(image) => uploaded_avatar = Some(image),
            Err(error) => {
                eprintln!("Avatar upload failed: {error}");

                return HttpResponse::BadRequest().json(json!({
                    "message": "Cannot upload avatar"
                }));
            }
        }
    }

    // Upload toàn bộ related_images mới.
    for file in related_files {
        match cloudinary
            .upload_related_image(file.bytes, &file.filename)
            .await
        {
            Ok(image) => uploaded_related.push(image),
            Err(error) => {
                eprintln!("Related image upload failed: {error}");

                cleanup_images(cloudinary.get_ref(), &uploaded_avatar, &uploaded_related).await;

                return HttpResponse::BadRequest().json(json!({
                    "message": "Cannot upload related image"
                }));
            }
        }
    }

    // Cập nhật user và thay thế danh sách ảnh liên quan.
    let result = user_service::update_user(
        pool.get_ref(),
        cloudinary.get_ref(),
        user_id,
        name,
        email,
        password_hash.as_deref(),
        phone_number,
        uploaded_avatar.as_ref(),
        &uploaded_related,
        related_images_field_present,
    )
    .await;

    match result {
        Ok(true) => match user_service::get_user(pool.get_ref(), user_id).await {
            Ok(Some(user)) => HttpResponse::Ok().json(json!({
                "message": "User updated successfully",
                "user": user
            })),

            Ok(None) => HttpResponse::InternalServerError().json(json!({
                "message": "User updated but could not be retrieved"
            })),

            Err(error) => {
                eprintln!("Get updated user failed: {error}");

                HttpResponse::InternalServerError().json(json!({
                    "message": "User updated but could not be retrieved"
                }))
            }
        },

        Ok(false) => {
            cleanup_images(cloudinary.get_ref(), &uploaded_avatar, &uploaded_related).await;

            HttpResponse::NotFound().json(json!({
                "message": "User not found"
            }))
        }

        Err(error) => {
            eprintln!("Update user failed: {error}");

            cleanup_images(cloudinary.get_ref(), &uploaded_avatar, &uploaded_related).await;

            HttpResponse::InternalServerError().json(json!({
                "message": "Cannot update user. Check that the email is unique and the database is available."
            }))
        }
    }
}

// // use crate::AppState;
// use crate::services::{
//     cloudinary_service::{CloudinaryService, UploadedImage},
//     user_service,
// };
// use actix_multipart::{Field, Multipart};
// use actix_web::{HttpResponse, Responder, web};
// use futures_util::StreamExt;
// use serde_json::json;
// use sqlx::PgPool;
// use std::collections::HashMap;
// use uuid::Uuid;

// const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
// const MAX_RELATED_IMAGES: usize = 20;

// struct PendingImage {
//     filename: String,
//     bytes: Vec<u8>,
// }

// async fn read_text_field(mut field: Field) -> Result<String, String> {
//     let mut bytes = Vec::new();

//     while let Some(chunk) = field.next().await {
//         let chunk = chunk.map_err(|e| e.to_string())?;

//         if bytes.len() + chunk.len() > 16 * 1024 {
//             return Err("Text field is too large".to_string());
//         }

//         bytes.extend_from_slice(&chunk);
//     }

//     let value =
//         String::from_utf8(bytes).map_err(|_| "Text field must be valid UTF-8".to_string())?;

//     Ok(value.trim().to_string())
// }

// async fn read_image_field(mut field: Field) -> Result<PendingImage, String> {
//     let filename = field
//         .content_disposition()
//         .and_then(|cd| cd.get_filename())
//         .map(|s| {
//             // Chỉ giữ tên file cuối cùng, không sử dụng đường dẫn client gửi.
//             std::path::Path::new(s)
//                 .file_name()
//                 .and_then(|n| n.to_str())
//                 .unwrap_or("image")
//                 .to_string()
//         })
//         .unwrap_or_else(|| "image".to_string());

//     let mut bytes = Vec::new();

//     while let Some(chunk) = field.next().await {
//         let chunk = chunk.map_err(|e| e.to_string())?;

//         if bytes.len() + chunk.len() > MAX_IMAGE_SIZE {
//             return Err("Each image must be 10 MB or smaller".to_string());
//         }

//         bytes.extend_from_slice(&chunk);
//     }

//     if bytes.is_empty() {
//         return Err("Image is empty".to_string());
//     }

//     Ok(PendingImage { filename, bytes })
// }

// async fn cleanup_images(
//     cloudinary: &CloudinaryService,
//     avatar: &Option<UploadedImage>,
//     related_images: &[UploadedImage],
// ) {
//     if let Some(image) = avatar {
//         if let Err(error) = cloudinary.destroy(&image.public_id).await {
//             eprintln!("Avatar cleanup failed: {error}");
//         }
//     }

//     for image in related_images {
//         if let Err(error) = cloudinary.destroy(&image.public_id).await {
//             eprintln!("Related image cleanup failed: {error}");
//         }
//     }
// }

// pub async fn create_user(
//     pool: web::Data<PgPool>,
//     cloudinary: web::Data<CloudinaryService>,
//     mut payload: Multipart,
// ) -> impl Responder {
//     let mut text_fields: HashMap<String, String> = HashMap::new();
//     let mut avatar_file: Option<PendingImage> = None;
//     let mut related_files: Vec<PendingImage> = Vec::new();

//     // Đọc toàn bộ multipart trước khi upload ảnh.
//     while let Some(item) = payload.next().await {
//         let field = match item {
//             Ok(field) => field,
//             Err(error) => {
//                 eprintln!("Multipart error: {error}");

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Invalid multipart data"
//                 }));
//             }
//         };

//         let field_name = match field.content_disposition().and_then(|cd| cd.get_name()) {
//             Some(name) => name.to_string(),
//             None => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Multipart field is missing a name"
//                 }));
//             }
//         };

//         match field_name.as_str() {
//             "name" | "email" | "password" | "phone_number" => {
//                 let value = match read_text_field(field).await {
//                     Ok(value) => value,
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };

//                 text_fields.insert(field_name, value);
//             }

//             "avatar" => {
//                 if avatar_file.is_some() {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Only one avatar is allowed"
//                     }));
//                 }

//                 avatar_file = match read_image_field(field).await {
//                     Ok(file) => Some(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };
//             }

//             "related_images" => {
//                 if related_files.len() >= MAX_RELATED_IMAGES {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Maximum 20 related images are allowed"
//                     }));
//                 }

//                 match read_image_field(field).await {
//                     Ok(file) => related_files.push(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 }
//             }

//             _ => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Unknown multipart field",
//                     "field": field_name
//                 }));
//             }
//         }
//     }

//     // Lấy dữ liệu text.
//     let name = text_fields.get("name").map(String::as_str).unwrap_or("");

//     let email = text_fields.get("email").map(String::as_str).unwrap_or("");

//     let password = text_fields
//         .get("password")
//         .map(String::as_str)
//         .unwrap_or("");

//     let phone_number = text_fields
//         .get("phone_number")
//         .filter(|value| !value.is_empty())
//         .map(String::as_str);

//     // Kiểm tra dữ liệu bắt buộc.
//     if name.is_empty() || email.is_empty() || password.is_empty() {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name, email and password are required"
//         }));
//     }

//     if password.len() < 8 {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "password must contain at least 8 bytes"
//         }));
//     }

//     if email.len() > 254 || name.len() > 200 {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name or email is too long"
//         }));
//     }

//     // Hash password.
//     let password_hash = match bcrypt::hash(password, bcrypt::DEFAULT_COST) {
//         Ok(hash) => hash,
//         Err(error) => {
//             eprintln!("Password hashing failed: {error}");

//             return HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot process password"
//             }));
//         }
//     };

//     // let cloudinary = &state.cloudinary;

//     let mut uploaded_avatar: Option<UploadedImage> = None;
//     let mut uploaded_related: Vec<UploadedImage> = Vec::new();

//     // Upload avatar vào CLOUDINARY_AVATAR_FOLDER.
//     if let Some(file) = avatar_file {
//         match cloudinary.upload_avatar(file.bytes, &file.filename).await {
//             Ok(image) => {
//                 uploaded_avatar = Some(image);
//             }

//             Err(error) => {
//                 eprintln!("Avatar upload failed: {error}");

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload avatar"
//                 }));
//             }
//         }
//     }

//     // Upload related images vào CLOUDINARY_RELATED_IMAGES_FOLDER.
//     for file in related_files {
//         match cloudinary
//             .upload_related_image(file.bytes, &file.filename)
//             .await
//         {
//             Ok(image) => {
//                 uploaded_related.push(image);
//             }

//             Err(error) => {
//                 eprintln!("Related image upload failed: {error}");

//                 // cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;

//                 cleanup_images(cloudinary.get_ref(), &uploaded_avatar, &uploaded_related).await;

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload related image"
//                 }));
//             }
//         }
//     }

//     // Tạo user và lưu thông tin ảnh vào database.
//     match user_service::create_user(
//         pool.get_ref(),
//         name,
//         email,
//         &password_hash,
//         phone_number,
//         uploaded_avatar.as_ref(),
//         &uploaded_related,
//     )
//     .await
//     {
//         Ok(user) => HttpResponse::Created().json(json!({
//             "message": "User created successfully",
//             "user": user
//         })),

//         Err(error) => {
//             eprintln!("Create user failed: {error}");

//             cleanup_images(cloudinary.get_ref(), &uploaded_avatar, &uploaded_related).await;

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot create user. Check that the email is unique and the database is available."
//             }))
//         }
//     }
// }

// pub async fn get_user(pool: web::Data<PgPool>, path: web::Path<String>) -> impl Responder {
//     let id = match Uuid::parse_str(&path.into_inner()) {
//         Ok(id) => id,
//         Err(_) => {
//             return HttpResponse::BadRequest().json(json!({
//                 "message": "Invalid user UUID"
//             }));
//         }
//     };

//     match user_service::get_user(pool.get_ref(), id).await {
//         Ok(Some(user)) => HttpResponse::Ok().json(user),

//         Ok(None) => HttpResponse::NotFound().json(json!({
//             "message": "User not found"
//         })),

//         Err(error) => {
//             eprintln!("Get user failed: {error}");

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot get user"
//             }))
//         }
//     }
// }

// pub async fn get_all_users(pool: web::Data<PgPool>) -> impl Responder {
//     match user_service::get_all_users(pool.get_ref()).await {
//         Ok(users) => HttpResponse::Ok().json(serde_json::json!({
//             "message": "Users retrieved successfully",
//             "count": users.len(),
//             "users": users
//         })),

//         Err(error) => {
//             eprintln!("Get all users failed: {error}");

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot get users"
//             }))
//         }
//     }
// }

// pub async fn delete_user(
//     pool: web::Data<PgPool>,
//     cloudinary: web::Data<CloudinaryService>,
//     path: web::Path<String>,
// ) -> impl Responder {
//     let user_id = match Uuid::parse_str(&path.into_inner()) {
//         Ok(id) => id,
//         Err(_) => {
//             return HttpResponse::BadRequest().json(json!({
//                 "message": "Invalid user UUID"
//             }));
//         }
//     };

//     match user_service::delete_user(pool.get_ref(), cloudinary.get_ref(), user_id).await {
//         Ok(true) => HttpResponse::Ok().json(json!({
//             "message": "User and associated images deleted successfully"
//         })),

//         Ok(false) => HttpResponse::NotFound().json(json!({
//             "message": "User not found"
//         })),

//         Err(error) => {
//             eprintln!("Delete user failed: {error}");

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot delete user"
//             }))
//         }
//     }
// }

// pub async fn update_user(
//     pool: web::Data<PgPool>,
//     cloudinary: web::Data<CloudinaryService>,
//     path: web::Path<String>,
//     mut payload: Multipart,
// ) -> impl Responder {
//     let user_id = match Uuid::parse_str(&path.into_inner()) {
//         Ok(id) => id,
//         Err(_) => {
//             return HttpResponse::BadRequest().json(json!({
//                 "message": "Invalid user UUID"
//             }));
//         }
//     };

//     let mut text_fields: HashMap<String, String> = HashMap::new();
//     let mut avatar_file: Option<PendingImage> = None;
//     let mut related_files: Vec<PendingImage> = Vec::new();
//     let mut related_images_field_present = false;

//     // Đọc toàn bộ multipart trước khi upload.
//     while let Some(item) = payload.next().await {
//         let field = match item {
//             Ok(field) => field,
//             Err(error) => {
//                 eprintln!("Multipart error: {error}");

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Invalid multipart data"
//                 }));
//             }
//         };

//         let field_name = match field.content_disposition().and_then(|cd| cd.get_name()) {
//             Some(name) => name.to_string(),
//             None => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Multipart field is missing a name"
//                 }));
//             }
//         };

//         match field_name.as_str() {
//             "name" | "email" | "password" | "phone_number" => {
//                 if text_fields.contains_key(&field_name) {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Duplicate text field",
//                         "field": field_name
//                     }));
//                 }

//                 let value = match read_text_field(field).await {
//                     Ok(value) => value,
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };

//                 text_fields.insert(field_name, value);
//             }

//             "avatar" => {
//                 if avatar_file.is_some() {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Only one avatar is allowed"
//                     }));
//                 }

//                 avatar_file = match read_image_field(field).await {
//                     Ok(file) => Some(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };
//             }

//             "related_images" => {
//                 related_images_field_present = true;

//                 if related_files.len() >= MAX_RELATED_IMAGES {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Maximum 20 related images are allowed"
//                     }));
//                 }

//                 match read_image_field(field).await {
//                     Ok(file) => related_files.push(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 }
//             }

//             _ => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Unknown multipart field",
//                     "field": field_name
//                 }));
//             }
//         }
//     }

//     let name = text_fields.get("name").map(String::as_str);
//     let email = text_fields.get("email").map(String::as_str);
//     let password = text_fields.get("password").map(String::as_str);

//     let phone_number = text_fields
//         .get("phone_number")
//         .filter(|value| !value.is_empty())
//         .map(String::as_str);

//     // Kiểm tra dữ liệu.
//     if name.is_some_and(|value| value.is_empty()) || email.is_some_and(|value| value.is_empty()) {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name and email cannot be empty"
//         }));
//     }

//     if password.is_some_and(|value| value.len() < 8) {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "password must contain at least 8 bytes"
//         }));
//     }

//     if name.is_some_and(|value| value.len() > 200) || email.is_some_and(|value| value.len() > 254) {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name or email is too long"
//         }));
//     }

//     // Hash password mới nếu có gửi.
//     let password_hash = match password {
//         Some(value) => match bcrypt::hash(value, bcrypt::DEFAULT_COST) {
//             Ok(hash) => Some(hash),
//             Err(error) => {
//                 eprintln!("Password hashing failed: {error}");

//                 return HttpResponse::InternalServerError().json(json!({
//                     "message": "Cannot process password"
//                 }));
//             }
//         },
//         None => None,
//     };

//     // let cloudinary = &state.cloudinary;

//     let mut uploaded_avatar: Option<UploadedImage> = None;
//     let mut uploaded_related: Vec<UploadedImage> = Vec::new();

//     // Upload avatar mới.
//     if let Some(file) = avatar_file {
//         match cloudinary.upload_avatar(file.bytes, &file.filename).await {
//             Ok(image) => uploaded_avatar = Some(image),
//             Err(error) => {
//                 eprintln!("Avatar upload failed: {error}");

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload avatar"
//                 }));
//             }
//         }
//     }

//     // Upload toàn bộ related_images mới.
//     for file in related_files {
//         match cloudinary
//             .upload_related_image(file.bytes, &file.filename)
//             .await
//         {
//             Ok(image) => uploaded_related.push(image),
//             Err(error) => {
//                 eprintln!("Related image upload failed: {error}");

//                 cleanup_images(cloudinary.get_ref(), &uploaded_avatar, &uploaded_related).await;

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload related image"
//                 }));
//             }
//         }
//     }

//     // Cập nhật user và thay thế danh sách ảnh liên quan.
//     let result = user_service::update_user(
//         pool.get_ref(),
//         cloudinary.get_ref(),
//         user_id,
//         name,
//         email,
//         password_hash.as_deref(),
//         phone_number,
//         uploaded_avatar.as_ref(),
//         &uploaded_related,
//         related_images_field_present,
//     )
//     .await;

//     match result {
//         Ok(true) => match user_service::get_user(pool.get_ref(), user_id).await {
//             Ok(Some(user)) => HttpResponse::Ok().json(json!({
//                 "message": "User updated successfully",
//                 "user": user
//             })),

//             Ok(None) => HttpResponse::InternalServerError().json(json!({
//                 "message": "User updated but could not be retrieved"
//             })),

//             Err(error) => {
//                 eprintln!("Get updated user failed: {error}");

//                 HttpResponse::InternalServerError().json(json!({
//                     "message": "User updated but could not be retrieved"
//                 }))
//             }
//         },

//         Ok(false) => {
//             cleanup_images(cloudinary.get_ref(), &uploaded_avatar, &uploaded_related).await;

//             HttpResponse::NotFound().json(json!({
//                 "message": "User not found"
//             }))
//         }

//         Err(error) => {
//             eprintln!("Update user failed: {error}");

//             cleanup_images(cloudinary.get_ref(), &uploaded_avatar, &uploaded_related).await;

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot update user. Check that the email is unique and the database is available."
//             }))
//         }
//     }
// }

// use crate::AppState;
// use crate::services::{
//     cloudinary_service::{CloudinaryService, UploadedImage},
//     user_service,
// };
// use actix_multipart::{Field, Multipart};
// use actix_web::{HttpResponse, Responder, web};
// use futures_util::StreamExt;
// use serde_json::json;
// use sqlx::PgPool;
// use std::collections::HashMap;
// use uuid::Uuid;

// const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
// const MAX_RELATED_IMAGES: usize = 20;

// struct PendingImage {
//     filename: String,
//     bytes: Vec<u8>,
// }

// async fn read_text_field(mut field: Field) -> Result<String, String> {
//     let mut bytes = Vec::new();

//     while let Some(chunk) = field.next().await {
//         let chunk = chunk.map_err(|e| e.to_string())?;

//         if bytes.len() + chunk.len() > 16 * 1024 {
//             return Err("Text field is too large".to_string());
//         }

//         bytes.extend_from_slice(&chunk);
//     }

//     let value =
//         String::from_utf8(bytes).map_err(|_| "Text field must be valid UTF-8".to_string())?;

//     Ok(value.trim().to_string())
// }

// async fn read_image_field(mut field: Field) -> Result<PendingImage, String> {
//     let filename = field
//         .content_disposition()
//         .and_then(|cd| cd.get_filename())
//         .map(|s| {
//             // Chỉ giữ tên file cuối cùng, không sử dụng đường dẫn client gửi.
//             std::path::Path::new(s)
//                 .file_name()
//                 .and_then(|n| n.to_str())
//                 .unwrap_or("image")
//                 .to_string()
//         })
//         .unwrap_or_else(|| "image".to_string());

//     let mut bytes = Vec::new();

//     while let Some(chunk) = field.next().await {
//         let chunk = chunk.map_err(|e| e.to_string())?;

//         if bytes.len() + chunk.len() > MAX_IMAGE_SIZE {
//             return Err("Each image must be 10 MB or smaller".to_string());
//         }

//         bytes.extend_from_slice(&chunk);
//     }

//     if bytes.is_empty() {
//         return Err("Image is empty".to_string());
//     }

//     Ok(PendingImage { filename, bytes })
// }

// async fn cleanup_images(
//     cloudinary: &CloudinaryService,
//     avatar: &Option<UploadedImage>,
//     related_images: &[UploadedImage],
// ) {
//     if let Some(image) = avatar {
//         if let Err(error) = cloudinary.destroy(&image.public_id).await {
//             eprintln!("Avatar cleanup failed: {error}");
//         }
//     }

//     for image in related_images {
//         if let Err(error) = cloudinary.destroy(&image.public_id).await {
//             eprintln!("Related image cleanup failed: {error}");
//         }
//     }
// }

// pub async fn create_user(
//     pool: web::Data<PgPool>,
//     state: web::Data<AppState>,
//     mut payload: Multipart,
// ) -> impl Responder {
//     let mut text_fields: HashMap<String, String> = HashMap::new();
//     let mut avatar_file: Option<PendingImage> = None;
//     let mut related_files: Vec<PendingImage> = Vec::new();

//     // Đọc toàn bộ multipart trước khi upload ảnh.
//     while let Some(item) = payload.next().await {
//         let field = match item {
//             Ok(field) => field,
//             Err(error) => {
//                 eprintln!("Multipart error: {error}");

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Invalid multipart data"
//                 }));
//             }
//         };

//         let field_name = match field.content_disposition().and_then(|cd| cd.get_name()) {
//             Some(name) => name.to_string(),
//             None => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Multipart field is missing a name"
//                 }));
//             }
//         };

//         match field_name.as_str() {
//             "name" | "email" | "password" | "phone_number" => {
//                 let value = match read_text_field(field).await {
//                     Ok(value) => value,
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };

//                 text_fields.insert(field_name, value);
//             }

//             "avatar" => {
//                 if avatar_file.is_some() {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Only one avatar is allowed"
//                     }));
//                 }

//                 avatar_file = match read_image_field(field).await {
//                     Ok(file) => Some(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };
//             }

//             "related_images" => {
//                 if related_files.len() >= MAX_RELATED_IMAGES {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Maximum 20 related images are allowed"
//                     }));
//                 }

//                 match read_image_field(field).await {
//                     Ok(file) => related_files.push(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 }
//             }

//             _ => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Unknown multipart field",
//                     "field": field_name
//                 }));
//             }
//         }
//     }

//     // Lấy dữ liệu text.
//     let name = text_fields.get("name").map(String::as_str).unwrap_or("");

//     let email = text_fields.get("email").map(String::as_str).unwrap_or("");

//     let password = text_fields
//         .get("password")
//         .map(String::as_str)
//         .unwrap_or("");

//     let phone_number = text_fields
//         .get("phone_number")
//         .filter(|value| !value.is_empty())
//         .map(String::as_str);

//     // Kiểm tra dữ liệu bắt buộc.
//     if name.is_empty() || email.is_empty() || password.is_empty() {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name, email and password are required"
//         }));
//     }

//     if password.len() < 8 {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "password must contain at least 8 bytes"
//         }));
//     }

//     if email.len() > 254 || name.len() > 200 {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name or email is too long"
//         }));
//     }

//     // Hash password.
//     let password_hash = match bcrypt::hash(password, bcrypt::DEFAULT_COST) {
//         Ok(hash) => hash,
//         Err(error) => {
//             eprintln!("Password hashing failed: {error}");

//             return HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot process password"
//             }));
//         }
//     };

//     let cloudinary = &state.cloudinary;

//     let mut uploaded_avatar: Option<UploadedImage> = None;
//     let mut uploaded_related: Vec<UploadedImage> = Vec::new();

//     // Upload avatar vào CLOUDINARY_AVATAR_FOLDER.
//     if let Some(file) = avatar_file {
//         match cloudinary.upload_avatar(file.bytes, &file.filename).await {
//             Ok(image) => {
//                 uploaded_avatar = Some(image);
//             }

//             Err(error) => {
//                 eprintln!("Avatar upload failed: {error}");

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload avatar"
//                 }));
//             }
//         }
//     }

//     // Upload related images vào CLOUDINARY_RELATED_IMAGES_FOLDER.
//     for file in related_files {
//         match cloudinary
//             .upload_related_image(file.bytes, &file.filename)
//             .await
//         {
//             Ok(image) => {
//                 uploaded_related.push(image);
//             }

//             Err(error) => {
//                 eprintln!("Related image upload failed: {error}");

//                 cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload related image"
//                 }));
//             }
//         }
//     }

//     // Tạo user và lưu thông tin ảnh vào database.
//     match user_service::create_user(
//         pool.get_ref(),
//         name,
//         email,
//         &password_hash,
//         phone_number,
//         uploaded_avatar.as_ref(),
//         &uploaded_related,
//     )
//     .await
//     {
//         Ok(user) => HttpResponse::Created().json(json!({
//             "message": "User created successfully",
//             "user": user
//         })),

//         Err(error) => {
//             eprintln!("Create user failed: {error}");

//             cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot create user. Check that the email is unique and the database is available."
//             }))
//         }
//     }
// }

// pub async fn get_user(pool: web::Data<PgPool>, path: web::Path<String>) -> impl Responder {
//     let id = match Uuid::parse_str(&path.into_inner()) {
//         Ok(id) => id,
//         Err(_) => {
//             return HttpResponse::BadRequest().json(json!({
//                 "message": "Invalid user UUID"
//             }));
//         }
//     };

//     match user_service::get_user(pool.get_ref(), id).await {
//         Ok(Some(user)) => HttpResponse::Ok().json(user),

//         Ok(None) => HttpResponse::NotFound().json(json!({
//             "message": "User not found"
//         })),

//         Err(error) => {
//             eprintln!("Get user failed: {error}");

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot get user"
//             }))
//         }
//     }
// }

// pub async fn get_all_users(pool: web::Data<PgPool>) -> impl Responder {
//     match user_service::get_all_users(pool.get_ref()).await {
//         Ok(users) => HttpResponse::Ok().json(serde_json::json!({
//             "message": "Users retrieved successfully",
//             "count": users.len(),
//             "users": users
//         })),

//         Err(error) => {
//             eprintln!("Get all users failed: {error}");

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot get users"
//             }))
//         }
//     }
// }

// pub async fn delete_user(
//     pool: web::Data<PgPool>,
//     state: web::Data<AppState>,
//     path: web::Path<String>,
// ) -> impl Responder {
//     let user_id = match Uuid::parse_str(&path.into_inner()) {
//         Ok(id) => id,
//         Err(_) => {
//             return HttpResponse::BadRequest().json(json!({
//                 "message": "Invalid user UUID"
//             }));
//         }
//     };

//     match user_service::delete_user(pool.get_ref(), &state.cloudinary, user_id).await {
//         Ok(true) => HttpResponse::Ok().json(json!({
//             "message": "User and associated images deleted successfully"
//         })),

//         Ok(false) => HttpResponse::NotFound().json(json!({
//             "message": "User not found"
//         })),

//         Err(error) => {
//             eprintln!("Delete user failed: {error}");

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot delete user"
//             }))
//         }
//     }
// }

// pub async fn update_user(
//     pool: web::Data<PgPool>,
//     state: web::Data<AppState>,
//     path: web::Path<String>,
//     mut payload: Multipart,
// ) -> impl Responder {
//     let user_id = match Uuid::parse_str(&path.into_inner()) {
//         Ok(id) => id,
//         Err(_) => {
//             return HttpResponse::BadRequest().json(json!({
//                 "message": "Invalid user UUID"
//             }));
//         }
//     };

//     let mut text_fields: HashMap<String, String> = HashMap::new();
//     let mut avatar_file: Option<PendingImage> = None;
//     let mut related_files: Vec<PendingImage> = Vec::new();
//     let mut related_images_field_present = false;

//     // Đọc toàn bộ multipart trước khi upload.
//     while let Some(item) = payload.next().await {
//         let field = match item {
//             Ok(field) => field,
//             Err(error) => {
//                 eprintln!("Multipart error: {error}");

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Invalid multipart data"
//                 }));
//             }
//         };

//         let field_name = match field.content_disposition().and_then(|cd| cd.get_name()) {
//             Some(name) => name.to_string(),
//             None => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Multipart field is missing a name"
//                 }));
//             }
//         };

//         match field_name.as_str() {
//             "name" | "email" | "password" | "phone_number" => {
//                 if text_fields.contains_key(&field_name) {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Duplicate text field",
//                         "field": field_name
//                     }));
//                 }

//                 let value = match read_text_field(field).await {
//                     Ok(value) => value,
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };

//                 text_fields.insert(field_name, value);
//             }

//             "avatar" => {
//                 if avatar_file.is_some() {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Only one avatar is allowed"
//                     }));
//                 }

//                 avatar_file = match read_image_field(field).await {
//                     Ok(file) => Some(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };
//             }

//             "related_images" => {
//                 related_images_field_present = true;

//                 if related_files.len() >= MAX_RELATED_IMAGES {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Maximum 20 related images are allowed"
//                     }));
//                 }

//                 match read_image_field(field).await {
//                     Ok(file) => related_files.push(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 }
//             }

//             _ => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Unknown multipart field",
//                     "field": field_name
//                 }));
//             }
//         }
//     }

//     let name = text_fields.get("name").map(String::as_str);
//     let email = text_fields.get("email").map(String::as_str);
//     let password = text_fields.get("password").map(String::as_str);

//     let phone_number = text_fields
//         .get("phone_number")
//         .filter(|value| !value.is_empty())
//         .map(String::as_str);

//     // Kiểm tra dữ liệu.
//     if name.is_some_and(|value| value.is_empty()) || email.is_some_and(|value| value.is_empty()) {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name and email cannot be empty"
//         }));
//     }

//     if password.is_some_and(|value| value.len() < 8) {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "password must contain at least 8 bytes"
//         }));
//     }

//     if name.is_some_and(|value| value.len() > 200) || email.is_some_and(|value| value.len() > 254) {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name or email is too long"
//         }));
//     }

//     // Hash password mới nếu có gửi.
//     let password_hash = match password {
//         Some(value) => match bcrypt::hash(value, bcrypt::DEFAULT_COST) {
//             Ok(hash) => Some(hash),
//             Err(error) => {
//                 eprintln!("Password hashing failed: {error}");

//                 return HttpResponse::InternalServerError().json(json!({
//                     "message": "Cannot process password"
//                 }));
//             }
//         },
//         None => None,
//     };

//     let cloudinary = &state.cloudinary;

//     let mut uploaded_avatar: Option<UploadedImage> = None;
//     let mut uploaded_related: Vec<UploadedImage> = Vec::new();

//     // Upload avatar mới.
//     if let Some(file) = avatar_file {
//         match cloudinary.upload_avatar(file.bytes, &file.filename).await {
//             Ok(image) => uploaded_avatar = Some(image),
//             Err(error) => {
//                 eprintln!("Avatar upload failed: {error}");

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload avatar"
//                 }));
//             }
//         }
//     }

//     // Upload toàn bộ related_images mới.
//     for file in related_files {
//         match cloudinary
//             .upload_related_image(file.bytes, &file.filename)
//             .await
//         {
//             Ok(image) => uploaded_related.push(image),
//             Err(error) => {
//                 eprintln!("Related image upload failed: {error}");

//                 cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload related image"
//                 }));
//             }
//         }
//     }

//     // Cập nhật user và thay thế danh sách ảnh liên quan.
//     let result = user_service::update_user(
//         pool.get_ref(),
//         cloudinary,
//         user_id,
//         name,
//         email,
//         password_hash.as_deref(),
//         phone_number,
//         uploaded_avatar.as_ref(),
//         &uploaded_related,
//         related_images_field_present,
//     )
//     .await;

//     match result {
//         Ok(true) => match user_service::get_user(pool.get_ref(), user_id).await {
//             Ok(Some(user)) => HttpResponse::Ok().json(json!({
//                 "message": "User updated successfully",
//                 "user": user
//             })),

//             Ok(None) => HttpResponse::InternalServerError().json(json!({
//                 "message": "User updated but could not be retrieved"
//             })),

//             Err(error) => {
//                 eprintln!("Get updated user failed: {error}");

//                 HttpResponse::InternalServerError().json(json!({
//                     "message": "User updated but could not be retrieved"
//                 }))
//             }
//         },

//         Ok(false) => {
//             cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;

//             HttpResponse::NotFound().json(json!({
//                 "message": "User not found"
//             }))
//         }

//         Err(error) => {
//             eprintln!("Update user failed: {error}");

//             cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot update user. Check that the email is unique and the database is available."
//             }))
//         }
//     }
// }

// pub async fn update_user(
//     pool: web::Data<PgPool>,
//     state: web::Data<AppState>,
//     path: web::Path<String>,
//     mut payload: Multipart,
// ) -> impl Responder {
//     let user_id = match Uuid::parse_str(&path.into_inner()) {
//         Ok(id) => id,
//         Err(_) => {
//             return HttpResponse::BadRequest().json(json!({
//                 "message": "Invalid user UUID"
//             }));
//         }
//     };

//     let mut text_fields: HashMap<String, String> = HashMap::new();
//     let mut avatar_file: Option<PendingImage> = None;
//     let mut related_files: Vec<PendingImage> = Vec::new();

//     // Đọc multipart trước khi upload ảnh.
//     while let Some(item) = payload.next().await {
//         let field = match item {
//             Ok(field) => field,
//             Err(error) => {
//                 eprintln!("Multipart error: {error}");
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Invalid multipart data"
//                 }));
//             }
//         };

//         let field_name = match field.content_disposition().and_then(|cd| cd.get_name()) {
//             Some(name) => name.to_string(),
//             None => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Multipart field is missing a name"
//                 }));
//             }
//         };

//         match field_name.as_str() {
//             "name" | "email" | "password" | "phone_number" => {
//                 if text_fields.contains_key(&field_name) {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Duplicate text field",
//                         "field": field_name
//                     }));
//                 }

//                 let value = match read_text_field(field).await {
//                     Ok(value) => value,
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };

//                 text_fields.insert(field_name, value);
//             }

//             "avatar" => {
//                 if avatar_file.is_some() {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Only one avatar is allowed"
//                     }));
//                 }

//                 avatar_file = match read_image_field(field).await {
//                     Ok(file) => Some(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };
//             }

//             "related_images" => {
//                 if related_files.len() >= MAX_RELATED_IMAGES {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Maximum 20 new related images are allowed"
//                     }));
//                 }

//                 match read_image_field(field).await {
//                     Ok(file) => related_files.push(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 }
//             }

//             _ => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Unknown multipart field",
//                     "field": field_name
//                 }));
//             }
//         }
//     }

//     let name = text_fields.get("name").map(String::as_str);
//     let email = text_fields.get("email").map(String::as_str);
//     let password = text_fields.get("password").map(String::as_str);

//     let phone_number = text_fields
//         .get("phone_number")
//         .filter(|value| !value.is_empty())
//         .map(String::as_str);

//     // Kiểm tra các trường được gửi lên.
//     if name.is_some_and(|value| value.is_empty()) || email.is_some_and(|value| value.is_empty()) {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name and email cannot be empty"
//         }));
//     }

//     if password.is_some_and(|value| value.len() < 8) {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "password must contain at least 8 bytes"
//         }));
//     }

//     if name.is_some_and(|value| value.len() > 200) || email.is_some_and(|value| value.len() > 254) {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name or email is too long"
//         }));
//     }

//     // Chỉ hash password khi client gửi password mới.
//     let password_hash = match password {
//         Some(value) => match bcrypt::hash(value, bcrypt::DEFAULT_COST) {
//             Ok(hash) => Some(hash),
//             Err(error) => {
//                 eprintln!("Password hashing failed: {error}");
//                 return HttpResponse::InternalServerError().json(json!({
//                     "message": "Cannot process password"
//                 }));
//             }
//         },
//         None => None,
//     };

//     let cloudinary = &state.cloudinary;

//     let mut uploaded_avatar: Option<UploadedImage> = None;
//     let mut uploaded_related: Vec<UploadedImage> = Vec::new();

//     // Upload avatar mới vào folder avatar.
//     if let Some(file) = avatar_file {
//         match cloudinary.upload_avatar(file.bytes, &file.filename).await {
//             Ok(image) => uploaded_avatar = Some(image),
//             Err(error) => {
//                 eprintln!("Avatar upload failed: {error}");
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload avatar"
//                 }));
//             }
//         }
//     }

//     // Upload các ảnh liên quan mới.
//     for file in related_files {
//         match cloudinary
//             .upload_related_image(file.bytes, &file.filename)
//             .await
//         {
//             Ok(image) => uploaded_related.push(image),
//             Err(error) => {
//                 eprintln!("Related image upload failed: {error}");

//                 cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;

//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload related image"
//                 }));
//             }
//         }
//     }

//     // Cập nhật database.
//     let result = user_service::update_user(
//         pool.get_ref(),
//         cloudinary,
//         user_id,
//         name,
//         email,
//         password_hash.as_deref(),
//         phone_number,
//         uploaded_avatar.as_ref(),
//         &uploaded_related,
//     )
//     .await;

//     match result {
//         Ok(true) => {
//             // Không xóa ảnh mới nếu bước đọc response thất bại:
//             // database có thể đã commit thành công.
//             match user_service::get_user(pool.get_ref(), user_id).await {
//                 Ok(Some(user)) => HttpResponse::Ok().json(json!({
//                     "message": "User updated successfully",
//                     "user": user
//                 })),
//                 Ok(None) => HttpResponse::InternalServerError().json(json!({
//                     "message": "User was updated but could not be retrieved"
//                 })),
//                 Err(error) => {
//                     eprintln!("Get updated user failed: {error}");
//                     HttpResponse::InternalServerError().json(json!({
//                         "message": "User was updated but could not be retrieved"
//                     }))
//                 }
//             }
//         }

//         Ok(false) => {
//             // User không tồn tại; dọn ảnh vừa upload.
//             cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;

//             HttpResponse::NotFound().json(json!({
//                 "message": "User not found"
//             }))
//         }

//         Err(error) => {
//             eprintln!("Update user failed: {error}");

//             // Nếu cập nhật thất bại, cố gắng xóa ảnh mới đã upload.
//             cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;

//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot update user. Check that the email is unique and the database is available."
//             }))
//         }
//     }
// }

// use crate::AppState;
// use crate::services::{
//     cloudinary_service::{CloudinaryService, UploadedImage},
//     user_service,
// };
// use actix_multipart::{Field, Multipart};
// use actix_web::{HttpResponse, Responder, web};
// use futures_util::StreamExt;
// use serde_json::json;
// use sqlx::PgPool;
// use std::collections::HashMap;
// use uuid::Uuid;
// const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
// const MAX_RELATED_IMAGES: usize = 20;
// struct PendingImage {
//     filename: String,
//     bytes: Vec<u8>,
// }
// async fn read_text_field(mut field: Field) -> Result<String, String> {
//     let mut bytes = Vec::new();
//     while let Some(chunk) = field.next().await {
//         let chunk = chunk.map_err(|e| e.to_string())?;
//         if bytes.len() + chunk.len() > 16 * 1024 {
//             return Err("Text field is too large".to_string());
//         }
//         bytes.extend_from_slice(&chunk);
//     }
//     let value =
//         String::from_utf8(bytes).map_err(|_| "Text field must be valid UTF-8".to_string())?;
//     Ok(value.trim().to_string())
// }
// async fn read_image_field(mut field: Field) -> Result<PendingImage, String> {
//     let filename = field
//         .content_disposition()
//         .and_then(|cd| cd.get_filename())
//         .map(|s| {
//             // Chỉ giữ tên file cuối cùng, không sử dụng đường dẫn client gửi.
//             std::path::Path::new(s)
//                 .file_name()
//                 .and_then(|n| n.to_str())
//                 .unwrap_or("image")
//                 .to_string()
//         })
//         .unwrap_or_else(|| "image".to_string());
//     let mut bytes = Vec::new();
//     while let Some(chunk) = field.next().await {
//         let chunk = chunk.map_err(|e| e.to_string())?;
//         if bytes.len() + chunk.len() > MAX_IMAGE_SIZE {
//             return Err("Each image must be 10 MB or smaller".to_string());
//         }
//         bytes.extend_from_slice(&chunk);
//     }
//     if bytes.is_empty() {
//         return Err("Image is empty".to_string());
//     }
//     Ok(PendingImage { filename, bytes })
// }
// async fn cleanup_images(
//     cloudinary: &CloudinaryService,
//     avatar: &Option<UploadedImage>,
//     related_images: &[UploadedImage],
// ) {
//     if let Some(image) = avatar {
//         if let Err(error) = cloudinary.destroy(&image.public_id).await {
//             eprintln!("Avatar cleanup failed: {error}");
//         }
//     }
//     for image in related_images {
//         if let Err(error) = cloudinary.destroy(&image.public_id).await {
//             eprintln!("Related image cleanup failed: {error}");
//         }
//     }
// }
// pub async fn create_user(
//     pool: web::Data<PgPool>,
//     state: web::Data<AppState>,
//     mut payload: Multipart,
// ) -> impl Responder {
//     let mut text_fields: HashMap<String, String> = HashMap::new();
//     let mut avatar_file: Option<PendingImage> = None;
//     let mut related_files: Vec<PendingImage> = Vec::new();
//     // Đọc toàn bộ multipart trước khi upload bất kỳ ảnh nào.
//     while let Some(item) = payload.next().await {
//         let field = match item {
//             Ok(field) => field,
//             Err(error) => {
//                 eprintln!("Multipart error: {error}");
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Invalid multipart data"
//                 }));
//             }
//         };
//         let field_name = match field.content_disposition().and_then(|cd| cd.get_name()) {
//             Some(name) => name.to_string(),
//             None => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Multipart field is missing a name"
//                 }));
//             }
//         };
//         match field_name.as_str() {
//             "name" | "email" | "password" | "phone_number" => {
//                 let value = match read_text_field(field).await {
//                     Ok(value) => value,
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };
//                 text_fields.insert(field_name, value);
//             }
//             "avatar" => {
//                 if avatar_file.is_some() {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Only one avatar is allowed"
//                     }));
//                 }
//                 avatar_file = match read_image_field(field).await {
//                     Ok(file) => Some(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };
//             }
//             "related_images" => {
//                 if related_files.len() >= MAX_RELATED_IMAGES {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Maximum 20 related images are allowed"
//                     }));
//                 }
//                 match read_image_field(field).await {
//                     Ok(file) => related_files.push(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 }
//             }
//             _ => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Unknown multipart field",
//                     "field": field_name
//                 }));
//             }
//         }
//     }
//     let name = text_fields.get("name").map(String::as_str).unwrap_or("");
//     let email = text_fields.get("email").map(String::as_str).unwrap_or("");
//     let password = text_fields
//         .get("password")
//         .map(String::as_str)
//         .unwrap_or("");
//     let phone_number = text_fields
//         .get("phone_number")
//         .filter(|value| !value.is_empty())
//         .map(String::as_str);
//     if name.is_empty() || email.is_empty() || password.is_empty() {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name, email and password are required"
//         }));
//     }
//     if password.len() < 8 {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "password must contain at least 8 bytes"
//         }));
//     }
//     if email.len() > 254 || name.len() > 200 {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name or email is too long"
//         }));
//     }
//     let password_hash = match bcrypt::hash(password, bcrypt::DEFAULT_COST) {
//         Ok(hash) => hash,
//         Err(error) => {
//             eprintln!("Password hashing failed: {error}");
//             return HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot process password"
//             }));
//         }
//     };
//     let cloudinary = &state.cloudinary;
//     let mut uploaded_avatar: Option<UploadedImage> = None;
//     let mut uploaded_related: Vec<UploadedImage> = Vec::new();
//     // Upload avatar nếu được gửi.
//     if let Some(file) = avatar_file {
//         // match cloudinary.upload(file.bytes, &file.filename).await {
//         //     Ok(image) => uploaded_avatar = Some(image),
//         //     Err(error) => {
//         //         eprintln!("Avatar upload failed: {error}");
//         //         return HttpResponse::BadRequest().json(json!({
//         //             "message": "Cannot upload avatar"
//         //         }));
//         //     }
//         // }
//         if let Some(file) = avatar_file {
//             match cloudinary.upload_avatar(file.bytes, &file.filename).await {
//                 Ok(image) => uploaded_avatar = Some(image),
//                 Err(error) => {
//                     eprintln!("Avatar upload failed: {error}");

//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Cannot upload avatar"
//                     }));
//                 }
//             }
//         }
//     }
//     // Upload lần lượt các ảnh liên quan.
//     for file in related_files {
//         match cloudinary.upload(file.bytes, &file.filename).await {
//             Ok(image) => uploaded_related.push(image),
//             Err(error) => {
//                 eprintln!("Related image upload failed: {error}");
//                 cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload related image"
//                 }));
//             }
//         }
//     }
//     match user_service::create_user(
//         pool.get_ref(),
//         name,
//         email,
//         &password_hash,
//         phone_number,
//         uploaded_avatar.as_ref(),
//         &uploaded_related,
//     )
//     .await
//     {
//         Ok(user) => HttpResponse::Created().json(json!({
//             "message": "User created successfully",
//             "user": user
//         })),
//         Err(error) => {
//             eprintln!("Create user failed: {error}");
//             cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;
//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot create user. Check that the email is unique and the database is available."
//             }))
//         }
//     }
// }
// pub async fn get_user(pool: web::Data<PgPool>, path: web::Path<String>) -> impl Responder {
//     let id = match Uuid::parse_str(&path.into_inner()) {
//         Ok(id) => id,
//         Err(_) => {
//             return HttpResponse::BadRequest().json(json!({
//                 "message": "Invalid user UUID"
//             }));
//         }
//     };
//     match user_service::get_user(pool.get_ref(), id).await {
//         Ok(Some(user)) => HttpResponse::Ok().json(user),
//         Ok(None) => HttpResponse::NotFound().json(json!({
//             "message": "User not found"
//         })),
//         Err(error) => {
//             eprintln!("Get user failed: {error}");
//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot get user"
//             }))
//         }
//     }
// }

// use crate::AppState;
// use crate::services::{
//     cloudinary_service::{CloudinaryService, UploadedImage},
//     user_service,
// };
// use actix_multipart::{Field, Multipart};
// use actix_web::{HttpResponse, Responder, web};
// use futures_util::StreamExt;
// use serde_json::json;
// use sqlx::PgPool;
// use std::collections::HashMap;
// use uuid::Uuid;
// const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
// const MAX_RELATED_IMAGES: usize = 20;
// struct PendingImage {
//     filename: String,
//     bytes: Vec<u8>,
// }
// async fn read_text_field(mut field: Field) -> Result<String, String> {
//     let mut bytes = Vec::new();
//     while let Some(chunk) = field.next().await {
//         let chunk = chunk.map_err(|e| e.to_string())?;
//         if bytes.len() + chunk.len() > 16 * 1024 {
//             return Err("Text field is too large".to_string());
//         }
//         bytes.extend_from_slice(&chunk);
//     }
//     let value =
//         String::from_utf8(bytes).map_err(|_| "Text field must be valid UTF-8".to_string())?;
//     Ok(value.trim().to_string())
// }
// async fn read_image_field(mut field: Field) -> Result<PendingImage, String> {
//     let filename = field
//         .content_disposition()
//         .and_then(|cd| cd.get_filename())
//         .map(|s| {
//             // Chỉ giữ tên file cuối cùng, không sử dụng đường dẫn client gửi.
//             std::path::Path::new(s)
//                 .file_name()
//                 .and_then(|n| n.to_str())
//                 .unwrap_or("image")
//                 .to_string()
//         })
//         .unwrap_or_else(|| "image".to_string());
//     let mut bytes = Vec::new();
//     while let Some(chunk) = field.next().await {
//         let chunk = chunk.map_err(|e| e.to_string())?;
//         if bytes.len() + chunk.len() > MAX_IMAGE_SIZE {
//             return Err("Each image must be 10 MB or smaller".to_string());
//         }
//         bytes.extend_from_slice(&chunk);
//     }
//     if bytes.is_empty() {
//         return Err("Image is empty".to_string());
//     }
//     Ok(PendingImage { filename, bytes })
// }
// async fn cleanup_images(
//     cloudinary: &CloudinaryService,
//     avatar: &Option<UploadedImage>,
//     related_images: &[UploadedImage],
// ) {
//     if let Some(image) = avatar {
//         if let Err(error) = cloudinary.destroy(&image.public_id).await {
//             eprintln!("Avatar cleanup failed: {error}");
//         }
//     }
//     for image in related_images {
//         if let Err(error) = cloudinary.destroy(&image.public_id).await {
//             eprintln!("Related image cleanup failed: {error}");
//         }
//     }
// }
// pub async fn create_user(
//     pool: web::Data<PgPool>,
//     state: web::Data<AppState>,
//     mut payload: Multipart,
// ) -> impl Responder {
//     let mut text_fields: HashMap<String, String> = HashMap::new();
//     let mut avatar_file: Option<PendingImage> = None;
//     let mut related_files: Vec<PendingImage> = Vec::new();
//     // Đọc toàn bộ multipart trước khi upload bất kỳ ảnh nào.
//     while let Some(item) = payload.next().await {
//         let field = match item {
//             Ok(field) => field,
//             Err(error) => {
//                 eprintln!("Multipart error: {error}");
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Invalid multipart data"
//                 }));
//             }
//         };
//         let field_name = match field.content_disposition().and_then(|cd| cd.get_name()) {
//             Some(name) => name.to_string(),
//             None => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Multipart field is missing a name"
//                 }));
//             }
//         };
//         match field_name.as_str() {
//             "name" | "email" | "password" | "phone_number" => {
//                 let value = match read_text_field(field).await {
//                     Ok(value) => value,
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };
//                 text_fields.insert(field_name, value);
//             }
//             "avatar" => {
//                 if avatar_file.is_some() {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Only one avatar is allowed"
//                     }));
//                 }
//                 avatar_file = match read_image_field(field).await {
//                     Ok(file) => Some(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 };
//             }
//             "related_images" => {
//                 if related_files.len() >= MAX_RELATED_IMAGES {
//                     return HttpResponse::BadRequest().json(json!({
//                         "message": "Maximum 20 related images are allowed"
//                     }));
//                 }
//                 match read_image_field(field).await {
//                     Ok(file) => related_files.push(file),
//                     Err(error) => {
//                         return HttpResponse::BadRequest().json(json!({
//                             "message": error
//                         }));
//                     }
//                 }
//             }
//             _ => {
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Unknown multipart field",
//                     "field": field_name
//                 }));
//             }
//         }
//     }
//     let name = text_fields.get("name").map(String::as_str).unwrap_or("");
//     let email = text_fields.get("email").map(String::as_str).unwrap_or("");
//     let password = text_fields
//         .get("password")
//         .map(String::as_str)
//         .unwrap_or("");
//     let phone_number = text_fields
//         .get("phone_number")
//         .filter(|value| !value.is_empty())
//         .map(String::as_str);
//     if name.is_empty() || email.is_empty() || password.is_empty() {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name, email and password are required"
//         }));
//     }
//     if password.len() < 8 {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "password must contain at least 8 bytes"
//         }));
//     }
//     if email.len() > 254 || name.len() > 200 {
//         return HttpResponse::BadRequest().json(json!({
//             "message": "name or email is too long"
//         }));
//     }
//     let password_hash = match bcrypt::hash(password, bcrypt::DEFAULT_COST) {
//         Ok(hash) => hash,
//         Err(error) => {
//             eprintln!("Password hashing failed: {error}");
//             return HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot process password"
//             }));
//         }
//     };
//     let cloudinary = &state.cloudinary;
//     let mut uploaded_avatar: Option<UploadedImage> = None;
//     let mut uploaded_related: Vec<UploadedImage> = Vec::new();
//     // Upload avatar nếu được gửi.
//     if let Some(file) = avatar_file {
//         match cloudinary.upload(file.bytes, &file.filename).await {
//             Ok(image) => uploaded_avatar = Some(image),
//             Err(error) => {
//                 eprintln!("Avatar upload failed: {error}");
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload avatar"
//                 }));
//             }
//         }
//     }
//     // Upload lần lượt các ảnh liên quan.
//     for file in related_files {
//         match cloudinary.upload(file.bytes, &file.filename).await {
//             Ok(image) => uploaded_related.push(image),
//             Err(error) => {
//                 eprintln!("Related image upload failed: {error}");
//                 cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;
//                 return HttpResponse::BadRequest().json(json!({
//                     "message": "Cannot upload related image"
//                 }));
//             }
//         }
//     }
//     match user_service::create_user(
//         pool.get_ref(),
//         name,
//         email,
//         &password_hash,
//         phone_number,
//         uploaded_avatar.as_ref(),
//         &uploaded_related,
//     )
//     .await
//     {
//         Ok(user) => HttpResponse::Created().json(json!({
//             "message": "User created successfully",
//             "user": user
//         })),
//         Err(error) => {
//             eprintln!("Create user failed: {error}");
//             cleanup_images(cloudinary, &uploaded_avatar, &uploaded_related).await;
//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot create user. Check that the email is unique and the database is available."
//             }))
//         }
//     }
// }
// pub async fn get_user(pool: web::Data<PgPool>, path: web::Path<String>) -> impl Responder {
//     let id = match Uuid::parse_str(&path.into_inner()) {
//         Ok(id) => id,
//         Err(_) => {
//             return HttpResponse::BadRequest().json(json!({
//                 "message": "Invalid user UUID"
//             }));
//         }
//     };
//     match user_service::get_user(pool.get_ref(), id).await {
//         Ok(Some(user)) => HttpResponse::Ok().json(user),
//         Ok(None) => HttpResponse::NotFound().json(json!({
//             "message": "User not found"
//         })),
//         Err(error) => {
//             eprintln!("Get user failed: {error}");
//             HttpResponse::InternalServerError().json(json!({
//                 "message": "Cannot get user"
//             }))
//         }
//     }
// }
