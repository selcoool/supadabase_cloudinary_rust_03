use crate::config::cloudinary::CloudinaryConfig;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use sha1::{Digest, Sha1};

const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;

#[derive(Clone)]
pub struct CloudinaryService {
    config: CloudinaryConfig,
    client: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct UploadedImage {
    pub url: String,
    pub public_id: String,
}

#[derive(Debug, Deserialize)]
struct CloudinaryUploadResponse {
    secure_url: String,
    public_id: String,
}

#[derive(Debug, Deserialize)]
struct CloudinaryDestroyResponse {
    result: String,
}

impl CloudinaryService {
    pub fn new(config: CloudinaryConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn sign(&self, parameters: &str) -> String {
        let input = format!("{parameters}{}", self.config.api_secret);
        let digest = Sha1::digest(input.as_bytes());
        hex::encode(digest)
    }

    fn validate_image(bytes: &[u8]) -> Result<(), String> {
        if bytes.is_empty() {
            return Err("Image is empty".to_string());
        }

        if bytes.len() > MAX_IMAGE_SIZE {
            return Err("Each image must be 10 MB or smaller".to_string());
        }

        let is_jpeg = bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
        let is_png = bytes.starts_with(b"\x89PNG\r\n\x1A\n");
        let is_gif = bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a");

        let is_webp = bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP";

        if !(is_jpeg || is_png || is_gif || is_webp) {
            return Err("Unsupported image. Use JPEG, PNG, GIF or WebP".to_string());
        }

        Ok(())
    }

    /// Upload vào folder mặc định CLOUDINARY_FOLDER.
    /// Giữ lại API này để tương thích với các nơi đang gọi upload cũ.
    // pub async fn upload(&self, bytes: Vec<u8>, filename: &str) -> Result<UploadedImage, String> {
    //     self.upload_to_folder(bytes, filename, &self.config.folder)
    //         .await
    // }

    /// Upload avatar vào CLOUDINARY_AVATAR_FOLDER.
    pub async fn upload_avatar(
        &self,
        bytes: Vec<u8>,
        filename: &str,
    ) -> Result<UploadedImage, String> {
        self.upload_to_folder(bytes, filename, &self.config.avatar_folder)
            .await
    }

    /// Upload ảnh liên quan vào CLOUDINARY_RELATED_IMAGES_FOLDER.
    pub async fn upload_related_image(
        &self,
        bytes: Vec<u8>,
        filename: &str,
    ) -> Result<UploadedImage, String> {
        self.upload_to_folder(bytes, filename, &self.config.related_images_folder)
            .await
    }

    /// Hàm upload dùng chung cho mọi folder.
    async fn upload_to_folder(
        &self,
        bytes: Vec<u8>,
        filename: &str,
        folder: &str,
    ) -> Result<UploadedImage, String> {
        Self::validate_image(&bytes)?;

        let folder = folder.trim_matches('/');

        if folder.is_empty() {
            return Err("Cloudinary folder cannot be empty".to_string());
        }

        let timestamp = chrono::Utc::now().timestamp().to_string();

        // Cloudinary yêu cầu các tham số được ký theo thứ tự alphabet.
        let parameters = format!("folder={}&timestamp={}", folder, timestamp);

        let signature = self.sign(&parameters);

        let part = Part::bytes(bytes)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| e.to_string())?;

        let form = Form::new()
            .text("api_key", self.config.api_key.clone())
            .text("timestamp", timestamp)
            .text("folder", folder.to_string())
            .text("signature", signature)
            .part("file", part);

        let url = format!(
            "https://api.cloudinary.com/v1_1/{}/image/upload",
            self.config.cloud_name
        );

        let response = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Cloudinary upload request failed: {e}"))?;

        let status = response.status();

        let body = response.text().await.map_err(|e| e.to_string())?;

        if !status.is_success() {
            eprintln!("Cloudinary upload error: {body}");
            return Err("Cloudinary rejected the image upload".to_string());
        }

        let result: CloudinaryUploadResponse =
            serde_json::from_str(&body).map_err(|e| format!("Invalid Cloudinary response: {e}"))?;

        Ok(UploadedImage {
            url: result.secure_url,
            public_id: result.public_id,
        })
    }

    /// Xóa ảnh dựa trên public_id đã lưu trong database.
    pub async fn destroy(&self, public_id: &str) -> Result<(), String> {
        let timestamp = chrono::Utc::now().timestamp().to_string();

        let parameters = format!("public_id={}&timestamp={}", public_id, timestamp);

        let signature = self.sign(&parameters);

        let form = Form::new()
            .text("api_key", self.config.api_key.clone())
            .text("public_id", public_id.to_string())
            .text("timestamp", timestamp)
            .text("signature", signature);

        let url = format!(
            "https://api.cloudinary.com/v1_1/{}/image/destroy",
            self.config.cloud_name
        );

        let response = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| format!("Cloudinary destroy request failed: {e}"))?;

        let status = response.status();

        let body = response.text().await.map_err(|e| e.to_string())?;

        if !status.is_success() {
            eprintln!("Cloudinary destroy error: {body}");
            return Err("Cannot delete image from Cloudinary".to_string());
        }

        let result: CloudinaryDestroyResponse =
            serde_json::from_str(&body).map_err(|e| format!("Invalid Cloudinary response: {e}"))?;

        if result.result != "ok" && result.result != "not found" {
            return Err(format!(
                "Cloudinary did not confirm image deletion: {}",
                result.result
            ));
        }

        Ok(())
    }
}

// use crate::config::cloudinary::CloudinaryConfig;
// use reqwest::multipart::{Form, Part};
// use serde::Deserialize;
// use sha1::{Digest, Sha1};
// const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024;
// #[derive(Clone)]
// pub struct CloudinaryService {
//     config: CloudinaryConfig,
//     client: reqwest::Client,
// }
// #[derive(Debug, Clone)]
// pub struct UploadedImage {
//     pub url: String,
//     pub public_id: String,
// }
// #[derive(Debug, Deserialize)]
// struct CloudinaryUploadResponse {
//     secure_url: String,
//     public_id: String,
// }
// #[derive(Debug, Deserialize)]
// struct CloudinaryDestroyResponse {
//     result: String,
// }
// impl CloudinaryService {
//     pub fn new(config: CloudinaryConfig) -> Self {
//         Self {
//             config,
//             client: reqwest::Client::new(),
//         }
//     }
//     fn sign(&self, parameters: &str) -> String {
//         let input = format!("{parameters}{}", self.config.api_secret);
//         let digest = Sha1::digest(input.as_bytes());
//         hex::encode(digest)
//     }
//     fn validate_image(bytes: &[u8]) -> Result<(), String> {
//         if bytes.is_empty() {
//             return Err("Image is empty".to_string());
//         }
//         if bytes.len() > MAX_IMAGE_SIZE {
//             return Err("Each image must be 10 MB or smaller".to_string());
//         }
//         let is_jpeg = bytes.starts_with(&[0xFF, 0xD8, 0xFF]);
//         let is_png = bytes.starts_with(b"\x89PNG\r\n\x1A\n");
//         let is_gif = bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a");
//         let is_webp = bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP";
//         if !(is_jpeg || is_png || is_gif || is_webp) {
//             return Err("Unsupported image. Use JPEG, PNG, GIF or WebP".to_string());
//         }
//         Ok(())
//     }
//     pub async fn upload(&self, bytes: Vec<u8>, filename: &str) -> Result<UploadedImage, String> {
//         Self::validate_image(&bytes)?;
//         let timestamp = chrono::Utc::now().timestamp().to_string();
//         let parameters = format!("folder={}&timestamp={}", self.config.folder, timestamp);
//         let signature = self.sign(&parameters);
//         let part = Part::bytes(bytes)
//             .file_name(filename.to_string())
//             .mime_str("application/octet-stream")
//             .map_err(|e| e.to_string())?;
//         let form = Form::new()
//             .text("api_key", self.config.api_key.clone())
//             .text("timestamp", timestamp)
//             .text("folder", self.config.folder.clone())
//             .text("signature", signature)
//             .part("file", part);
//         let url = format!(
//             "https://api.cloudinary.com/v1_1/{}/image/upload",
//             self.config.cloud_name
//         );
//         let response = self
//             .client
//             .post(url)
//             .multipart(form)
//             .send()
//             .await
//             .map_err(|e| format!("Cloudinary upload request failed: {e}"))?;
//         let status = response.status();
//         let body = response.text().await.map_err(|e| e.to_string())?;
//         if !status.is_success() {
//             eprintln!("Cloudinary upload error: {body}");
//             return Err("Cloudinary rejected the image upload".to_string());
//         }
//         let result: CloudinaryUploadResponse =
//             serde_json::from_str(&body).map_err(|e| format!("Invalid Cloudinary response: {e}"))?;
//         Ok(UploadedImage {
//             url: result.secure_url,
//             public_id: result.public_id,
//         })
//     }
//     pub async fn destroy(&self, public_id: &str) -> Result<(), String> {
//         let timestamp = chrono::Utc::now().timestamp().to_string();
//         let parameters = format!("public_id={}&timestamp={}", public_id, timestamp);
//         let signature = self.sign(&parameters);
//         let form = Form::new()
//             .text("api_key", self.config.api_key.clone())
//             .text("public_id", public_id.to_string())
//             .text("timestamp", timestamp)
//             .text("signature", signature);
//         let url = format!(
//             "https://api.cloudinary.com/v1_1/{}/image/destroy",
//             self.config.cloud_name
//         );
//         let response = self
//             .client
//             .post(url)
//             .multipart(form)
//             .send()
//             .await
//             .map_err(|e| format!("Cloudinary destroy request failed: {e}"))?;
//         let status = response.status();
//         let body = response.text().await.map_err(|e| e.to_string())?;
//         if !status.is_success() {
//             eprintln!("Cloudinary destroy error: {body}");
//             return Err("Cannot delete image from Cloudinary".to_string());
//         }
//         let result: CloudinaryDestroyResponse =
//             serde_json::from_str(&body).map_err(|e| format!("Invalid Cloudinary response: {e}"))?;
//         if result.result != "ok" && result.result != "not found" {
//             return Err(format!(
//                 "Cloudinary did not confirm image deletion: {}",
//                 result.result
//             ));
//         }
//         Ok(())
//     }
// }
