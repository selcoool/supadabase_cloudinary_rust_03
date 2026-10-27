use crate::services::cloudinary_service::{CloudinaryService, UploadedImage};
use actix_multipart::Field;
use futures_util::StreamExt;
use std::path::Path;

pub const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
pub const MAX_RELATED_IMAGES: usize = 20;
const MAX_TEXT_FIELD_SIZE: usize = 16 * 1024;

pub struct PendingImage {
    pub filename: String,
    pub bytes: Vec<u8>,
}

pub async fn read_text_field(mut field: Field) -> Result<String, String> {
    let mut bytes = Vec::new();

    while let Some(chunk) = field.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;

        if bytes.len() + chunk.len() > MAX_TEXT_FIELD_SIZE {
            return Err("Text field is too large".to_string());
        }

        bytes.extend_from_slice(&chunk);
    }

    let value =
        String::from_utf8(bytes).map_err(|_| "Text field must be valid UTF-8".to_string())?;

    Ok(value.trim().to_string())
}

pub async fn read_image_field(mut field: Field) -> Result<PendingImage, String> {
    let filename = field
        .content_disposition()
        .and_then(|cd| cd.get_filename())
        .map(|s| {
            Path::new(s)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("image")
                .to_string()
        })
        .unwrap_or_else(|| "image".to_string());

    let mut bytes = Vec::new();

    while let Some(chunk) = field.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;

        if bytes.len() + chunk.len() > MAX_IMAGE_SIZE {
            return Err("Each image must be 10 MB or smaller".to_string());
        }

        bytes.extend_from_slice(&chunk);
    }

    if bytes.is_empty() {
        return Err("Image is empty".to_string());
    }

    Ok(PendingImage { filename, bytes })
}

pub async fn cleanup_images(
    cloudinary: &CloudinaryService,
    avatar: &Option<UploadedImage>,
    related_images: &[UploadedImage],
) {
    if let Some(image) = avatar {
        if let Err(error) = cloudinary.destroy(&image.public_id).await {
            eprintln!("Avatar cleanup failed: {error}");
        }
    }

    for image in related_images {
        if let Err(error) = cloudinary.destroy(&image.public_id).await {
            eprintln!("Related image cleanup failed: {error}");
        }
    }
}
