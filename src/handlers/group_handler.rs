use actix_web::{HttpRequest, HttpResponse, Responder, web};
use serde_json::json;
use uuid::Uuid;

use crate::{
    models::group::{AddGroupMembers, CreateGroup},
    services::{group_service, message_service::AppState},
};

// =====================================
// ADD GROUP MEMBERS
// =====================================

pub async fn add_group_members(
    req: HttpRequest,
    state: web::Data<AppState>,
    path: web::Path<i64>,
    input: web::Json<AddGroupMembers>,
) -> impl Responder {
    // Lấy UUID người thực hiện từ header
    // DEMO ONLY: Sau này thay bằng JWT
    let admin_id = match req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| Uuid::parse_str(v).ok())
    {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(json!({
                "success": false,
                "message": "Unauthorized"
            }));
        }
    };

    let group_id = path.into_inner();
    let user_ids = input.into_inner().user_ids;

    dbg!(&admin_id);
    dbg!(&user_ids);

    // TODO: Kiểm tra admin_id có quyền thêm thành viên vào group.

    match group_service::add_group_members(&state.pool, group_id, user_ids).await {
        Ok(added) => HttpResponse::Ok().json(json!({
            "success": true,
            "message": "Thêm thành viên thành công",
            "group_id": group_id,
            "added_count": added
        })),

        Err(error) => HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": error
        })),
    }
}

// =====================================
// CREATE GROUP
// =====================================

pub async fn create_group(
    state: web::Data<AppState>,
    input: web::Json<CreateGroup>,
) -> impl Responder {
    match group_service::create_group(&state.pool, input.into_inner()).await {
        Ok(group) => HttpResponse::Created().json(json!({
            "success": true,
            "message": "Tạo group thành công",
            "data": group
        })),

        Err(error) => HttpResponse::BadRequest().json(json!({
            "success": false,
            "message": error
        })),
    }
}

// // use actix_web::{HttpRequest, HttpResponse, Responder, web};
// // use serde_json::json;

// // use crate::{
// //     models::group::AddGroupMembers, services::group_service, services::message_service::AppState,
// // };

// use actix_web::{HttpRequest, HttpResponse, Responder, web};
// use serde_json::json;

// use crate::{
//     models::group::{AddGroupMembers, CreateGroup},
//     services::{group_service, message_service::AppState},
// };

// pub async fn add_group_members(
//     req: HttpRequest,
//     state: web::Data<AppState>,
//     path: web::Path<i64>,
//     input: web::Json<AddGroupMembers>,
// ) -> impl Responder {
//     // DEMO ONLY: thay bằng JWT authentication
//     let admin_id = match req
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

//     let group_id = path.into_inner();
//     let user_ids = input.into_inner().user_ids;
//     dbg!(&user_ids);
//     // TODO: Kiểm tra admin_id có quyền thêm thành viên vào group.

//     match group_service::add_group_members(&state.pool, group_id, user_ids).await {
//         Ok(added) => HttpResponse::Ok().json(json!({
//             "success": true,
//             "message": "Thêm thành viên thành công",
//             "group_id": group_id,
//             "added_count": added
//         })),

//         Err(error) => HttpResponse::BadRequest().json(json!({
//             "success": false,
//             "message": error
//         })),
//     }
// }

// pub async fn create_group(
//     state: web::Data<AppState>,
//     input: web::Json<CreateGroup>,
// ) -> impl Responder {
//     match group_service::create_group(&state.pool, input.into_inner()).await {
//         Ok(group) => HttpResponse::Created().json(json!({
//             "success": true,
//             "message": "Tạo group thành công",
//             "data": group
//         })),

//         Err(error) => HttpResponse::BadRequest().json(json!({
//             "success": false,
//             "message": error
//         })),
//     }
// }
