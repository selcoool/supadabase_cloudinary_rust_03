use std::env;

#[derive(Clone)]
pub struct CloudinaryConfig {
    pub cloud_name: String,
    pub api_key: String,
    pub api_secret: String,
    pub avatar_folder: String,
    pub related_images_folder: String,
}

impl CloudinaryConfig {
    pub fn from_env() -> Result<Self, String> {
        let cloud_name = env::var("CLOUDINARY_CLOUD_NAME")
            .map_err(|_| "Missing CLOUDINARY_CLOUD_NAME".to_string())?;

        let api_key =
            env::var("CLOUDINARY_API_KEY").map_err(|_| "Missing CLOUDINARY_API_KEY".to_string())?;

        let api_secret = env::var("CLOUDINARY_API_SECRET")
            .map_err(|_| "Missing CLOUDINARY_API_SECRET".to_string())?;

        if cloud_name.trim().is_empty() || api_key.trim().is_empty() || api_secret.trim().is_empty()
        {
            return Err("Cloudinary credentials cannot be empty".to_string());
        }

        let folder = env::var("CLOUDINARY_FOLDER").unwrap_or_else(|_| "users".to_string());

        if folder.trim().is_empty() {
            return Err("CLOUDINARY_FOLDER cannot be empty".to_string());
        }

        let base_folder = folder.trim_matches('/');

        let avatar_folder = env::var("CLOUDINARY_AVATAR_FOLDER")
            .unwrap_or_else(|_| format!("{}/avatars", base_folder));

        let related_images_folder = env::var("CLOUDINARY_RELATED_IMAGES_FOLDER")
            .unwrap_or_else(|_| format!("{}/related_images", base_folder));

        if avatar_folder.trim().is_empty() || related_images_folder.trim().is_empty() {
            return Err("Cloudinary folders cannot be empty".to_string());
        }

        Ok(Self {
            cloud_name,
            api_key,
            api_secret,
            avatar_folder: avatar_folder.trim_matches('/').to_string(),
            related_images_folder: related_images_folder.trim_matches('/').to_string(),
        })
    }
}

// use std::env;

// #[derive(Clone)]
// pub struct CloudinaryConfig {
//     pub cloud_name: String,
//     pub api_key: String,
//     pub api_secret: String,
//     pub folder: String,
//     pub avatar_folder: String,
//     pub related_images_folder: String,
// }

// impl CloudinaryConfig {
//     pub fn from_env() -> Result<Self, String> {
//         let cloud_name = env::var("CLOUDINARY_CLOUD_NAME")
//             .map_err(|_| "Missing CLOUDINARY_CLOUD_NAME".to_string())?;

//         let api_key =
//             env::var("CLOUDINARY_API_KEY").map_err(|_| "Missing CLOUDINARY_API_KEY".to_string())?;

//         let api_secret = env::var("CLOUDINARY_API_SECRET")
//             .map_err(|_| "Missing CLOUDINARY_API_SECRET".to_string())?;

//         if cloud_name.trim().is_empty() || api_key.trim().is_empty() || api_secret.trim().is_empty()
//         {
//             return Err("Cloudinary credentials cannot be empty".to_string());
//         }

//         let folder = env::var("CLOUDINARY_FOLDER").unwrap_or_else(|_| "users".to_string());

//         if folder.trim().is_empty() {
//             return Err("CLOUDINARY_FOLDER cannot be empty".to_string());
//         }

//         let avatar_folder = env::var("CLOUDINARY_AVATAR_FOLDER")
//             .unwrap_or_else(|_| format!("{}/avatars", folder.trim_matches('/')));

//         let related_images_folder = env::var("CLOUDINARY_RELATED_IMAGES_FOLDER")
//             .unwrap_or_else(|_| format!("{}/related_images", folder.trim_matches('/')));

//         if avatar_folder.trim().is_empty() || related_images_folder.trim().is_empty() {
//             return Err("Cloudinary folders cannot be empty".to_string());
//         }

//         Ok(Self {
//             cloud_name,
//             api_key,
//             api_secret,
//             folder: folder.trim_matches('/').to_string(),
//             avatar_folder: avatar_folder.trim_matches('/').to_string(),
//             related_images_folder: related_images_folder.trim_matches('/').to_string(),
//         })
//     }
// }

// #[derive(Clone)]
// pub struct CloudinaryConfig {
//     pub cloud_name: String,
//     pub api_key: String,
//     pub api_secret: String,
//     pub folder: String,
// }

// impl CloudinaryConfig {
//     pub fn from_env() -> Result<Self, String> {
//         let cloud_name =
//             std::env::var("CLOUDINARY_CLOUD_NAME").map_err(|_| "Missing CLOUDINARY_CLOUD_NAME")?;

//         let api_key =
//             std::env::var("CLOUDINARY_API_KEY").map_err(|_| "Missing CLOUDINARY_API_KEY")?;

//         let api_secret =
//             std::env::var("CLOUDINARY_API_SECRET").map_err(|_| "Missing CLOUDINARY_API_SECRET")?;

//         if cloud_name.trim().is_empty() || api_key.trim().is_empty() || api_secret.trim().is_empty()
//         {
//             return Err("Cloudinary credentials cannot be empty".to_string());
//         }

//         Ok(Self {
//             cloud_name,
//             api_key,
//             api_secret,
//             folder: "users".to_string(),
//         })
//     }
// }
