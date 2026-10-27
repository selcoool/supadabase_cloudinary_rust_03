mod config {
    pub mod cloudinary;
    pub mod database;
}

mod models {
    pub mod user;
}

mod utils {
    pub mod upload_helpers;
}

mod repositories {
    pub mod user_image_repository;
    pub mod user_repository;
}

mod services {
    pub mod cloudinary_service;
    pub mod user_service;
}

mod handlers {
    pub mod user_handler;
}

mod routes {
    pub mod user_routes;
}

use actix_web::{App, HttpServer, web};
use config::cloudinary::CloudinaryConfig;
use config::database::create_pool;
use routes::user_routes::configure_routes;
use services::cloudinary_service::CloudinaryService;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();

    // 1. Khởi tạo PostgreSQL connection pool
    let pool = create_pool()
        .await
        .expect("Cannot connect to Supabase PostgreSQL");

    // 2. Khởi tạo Cloudinary service
    let cloudinary_config =
        CloudinaryConfig::from_env().expect("Cloudinary configuration is invalid");

    let cloudinary_service = CloudinaryService::new(cloudinary_config);

    // 3. Đăng ký dependency riêng biệt
    let pool = web::Data::new(pool);
    let cloudinary = web::Data::new(cloudinary_service);

    // 4. Cấu hình host và port
    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    let port: u16 = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()
        .expect("SERVER_PORT must be a valid port");

    println!("Server running at http://{host}:{port}");

    // 5. Khởi chạy server
    HttpServer::new(move || {
        App::new()
            .app_data(pool.clone())
            .app_data(cloudinary.clone())
            .configure(configure_routes)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

// mod config {
//     pub mod database;
//     pub mod cloudinary;
// }

// mod models {
//     pub mod user;
// }

// mod repositories {
//     pub mod user_repository;
//     pub mod user_image_repository;
// }

// mod services {
//     pub mod cloudinary_service;
//     pub mod user_service;
// }

// mod handlers {
//     pub mod user_handler;
// }

// mod routes {
//     pub mod user_routes;
// }

// use actix_web::{App, HttpServer, web};
// use config::cloudinary::CloudinaryConfig;
// use config::database::create_pool;
// use routes::user_routes::configure_routes;
// use services::cloudinary_service::CloudinaryService;

// #[derive(Clone)]
// pub struct AppState {
//     pub cloudinary: CloudinaryService,
// }

// #[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     dotenvy::dotenv().ok();

//     let pool = create_pool()
//         .await
//         .expect("Cannot connect to Supabase PostgreSQL");

//     let cloudinary_config = CloudinaryConfig::from_env()
//         .expect("Cloudinary configuration is invalid");

//     let cloudinary = CloudinaryService::new(cloudinary_config);

//     let state = web::Data::new(AppState { cloudinary });

//     let host = std::env::var("SERVER_HOST")
//         .unwrap_or_else(|_| "127.0.0.1".to_string());

//     let port: u16 = std::env::var("SERVER_PORT")
//         .unwrap_or_else(|_| "8080".to_string())
//         .parse()
//         .expect("SERVER_PORT must be a valid port");

//     println!("Server running at http://{host}:{port}");

//     HttpServer::new(move || {
//         App::new()
//             .app_data(web::Data::new(pool.clone()))
//             .app_data(state.clone())
//             .configure(configure_routes)
//     })
//     .bind((host, port))?
//     .run()
//     .await
// }
