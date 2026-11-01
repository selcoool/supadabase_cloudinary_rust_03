use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde_json::json;
use uuid::Uuid;

use crate::{
    models::message::CreateMessage,
    services::message_service::{self, AppState},
};

pub async fn create_message(
    req: HttpRequest,
    state: web::Data<AppState>,
    input: web::Json<CreateMessage>,
) -> impl Responder {
    // DEMO ONLY: Sau này thay bằng JWT authentication.
    // Lấy sender_id từ header x-user-id.
    let sender_id = match req
        .headers()
        .get("x-user-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
    {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(json!({
                "success": false,
                "message": "Invalid or missing x-user-id"
            }));
        }
    };

    // Gửi message đến service
    match message_service::send_message(&state, sender_id, input.into_inner()).await {
        Ok(message) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "Message sent successfully",
            "data": message
        })),

        Err(error) => HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": error
        })),
    }
}

// use actix_web::{HttpRequest, HttpResponse, Responder, web};
// use serde_json::json;

// use crate::{
//     models::message::CreateMessage,
//     services::message_service::{self, AppState},
// };

// pub async fn create_message(
//     req: HttpRequest,
//     state: web::Data<AppState>,
//     input: web::Json<CreateMessage>,
// ) -> impl Responder {
//     // DEMO ONLY: thay bằng JWT authentication khi triển khai thực tế.
//     let sender_id = match req
//         .headers()
//         .get("x-user-id")
//         .and_then(|v| v.to_str().ok())
//         .and_then(|v| v.parse::<i64>().ok())
//     {
//         Some(id) => id,
//         None => {
//             return HttpResponse::Unauthorized().json(json!({
//                 "success": false,
//                 "message": "Unauthorized"
//             }));
//         }
//     };

//     match message_service::send_message(&state, sender_id, input.into_inner()).await {
//         Ok(message) => HttpResponse::Ok().json(json!({
//             "success": true,
//             "data": message
//         })),

//         Err(error) => HttpResponse::BadRequest().json(json!({
//             "success": false,
//             "message": error
//         })),
//     }
// }

// use actix_web::{HttpRequest, HttpResponse, web};
// use serde_json::json;

// use crate::{
//     models::message::CreateMessage, services::message_service, services::message_service::AppState,
// };

// pub async fn create_message(
//     req: HttpRequest,
//     state: web::Data<AppState>,
//     input: web::Json<CreateMessage>,
// ) -> HttpResponse {
//     // DEMO ONLY: thay bằng JWT authentication khi triển khai thực tế.
//     let sender_id = match req
//         .headers()
//         .get("x-user-id")
//         .and_then(|v| v.to_str().ok())
//         .and_then(|v| v.parse::<i64>().ok())
//     {
//         Some(id) => id,
//         None => {
//             return HttpResponse::Unauthorized().json(json!({
//                 "success": false,
//                 "message": "Unauthorized"
//             }));
//         }
//     };

//     match message_service::send_message(&state, sender_id, input.into_inner()).await {
//         Ok(message) => HttpResponse::Ok().json(json!({
//             "success": true,
//             "data": message
//         })),

//         Err(error) => HttpResponse::BadRequest().json(json!({
//             "success": false,
//             "message": error
//         })),
//     }
// }
