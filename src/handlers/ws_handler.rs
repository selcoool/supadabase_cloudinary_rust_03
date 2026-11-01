use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_ws::Message;
use futures_util::StreamExt;
use serde_json::json;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{repositories::message_repository, services::message_service::AppState};

pub async fn websocket(
    req: HttpRequest,
    body: web::Payload,
    state: web::Data<AppState>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, Error> {
    let user_id = path.into_inner();

    // 1. Kiểm tra user tồn tại
    let user_exists =
        sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1)")
            .bind(user_id)
            .fetch_one(&state.pool)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;

    if !user_exists {
        return Ok(HttpResponse::NotFound().body("User not found"));
    }

    // 2. Lấy lịch sử Private Chat
    let private_messages = message_repository::get_private_messages(&state.pool, user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // 3. Lấy lịch sử Group Chat
    let group_messages = message_repository::get_group_messages(&state.pool, user_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // 4. Lấy lịch sử All Chat
    let all_messages = message_repository::get_all_messages(&state.pool)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // 5. Tạo 3 event riêng biệt
    let history_messages = vec![
        json!({
            "event": "private_history",
            "data": private_messages
        }),
        json!({
            "event": "group_history",
            "data": group_messages
        }),
        json!({
            "event": "all_history",
            "data": all_messages
        }),
    ];

    // Chuyển JSON thành String
    let history_messages: Vec<String> = history_messages
        .into_iter()
        .map(|item| serde_json::to_string(&item).unwrap())
        .collect();

    // 6. Khởi tạo WebSocket
    let (response, session, mut stream) = actix_ws::handle(&req, body)?;

    // 7. Tạo channel
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // 8. Đăng ký vào Hub
    let connection_id = state.hub.lock().await.connect(user_id, tx);

    let state_clone = state.clone();

    // 9. Chạy WebSocket
    actix_web::rt::spawn(async move {
        let mut session = session;

        println!("User {} connected", user_id);

        // GỬI 3 LOẠI LỊCH SỬ KHI KẾT NỐI
        for message in history_messages {
            if session.text(message).await.is_err() {
                break;
            }
        }

        // NHẬN VÀ GỬI REALTIME
        loop {
            tokio::select! {
                // Nhận message từ client
                incoming = stream.next() => {
                    match incoming {
                        Some(Ok(Message::Ping(bytes))) => {
                            if session.pong(&bytes).await.is_err() {
                                break;
                            }
                        }

                        Some(Ok(Message::Close(reason))) => {
                            let _ = session.close(reason).await;
                            break;
                        }

                        Some(Ok(Message::Text(text))) => {
                            println!(
                                "Received from {}: {}",
                                user_id,
                                text
                            );
                        }

                        Some(Ok(_)) => {}

                        Some(Err(_)) | None => break,
                    }
                }

                // Nhận message từ Hub
                outgoing = rx.recv() => {
                    match outgoing {
                        Some(message) => {
                            if session.text(message).await.is_err() {
                                break;
                            }
                        }

                        None => break,
                    }
                }
            }
        }

        // 10. Ngắt kết nối
        state_clone
            .hub
            .lock()
            .await
            .disconnect(user_id, connection_id);

        println!("User {} disconnected", user_id);
    });

    Ok(response)
}

// use actix_web::{Error, HttpRequest, HttpResponse, web};
// use actix_ws::Message;
// use futures_util::StreamExt;
// use tokio::sync::mpsc;
// use uuid::Uuid;

// use crate::services::message_service::AppState;

// pub async fn websocket(
//     req: HttpRequest,
//     body: web::Payload,
//     state: web::Data<AppState>,
//     path: web::Path<Uuid>,
// ) -> Result<HttpResponse, Error> {
//     // DEMO: user_id lấy từ URL.
//     // Production phải lấy từ JWT đã xác thực.
//     let user_id = path.into_inner();

//     // Khởi tạo WebSocket connection
//     let (response, session, mut stream) = actix_ws::handle(&req, body)?;

//     // Channel gửi message đến client
//     let (tx, mut rx) = mpsc::unbounded_channel::<String>();

//     // Đăng ký kết nối vào Hub
//     let connection_id = state.hub.lock().await.connect(user_id, tx);

//     let state_clone = state.clone();

//     actix_web::rt::spawn(async move {
//         let mut session = session;

//         loop {
//             tokio::select! {
//                 // Nhận message từ client
//                 incoming = stream.next() => {
//                     match incoming {
//                         Some(Ok(Message::Ping(bytes))) => {
//                             if session.pong(&bytes).await.is_err() {
//                                 break;
//                             }
//                         }

//                         Some(Ok(Message::Close(reason))) => {
//                             let _ = session.close(reason).await;
//                             break;
//                         }

//                         Some(Ok(_)) => {}

//                         Some(Err(_)) | None => break,
//                     }
//                 }

//                 // Nhận message từ Hub để gửi đến client
//                 outgoing = rx.recv() => {
//                     match outgoing {
//                         Some(message) => {
//                             if session.text(message).await.is_err() {
//                                 break;
//                             }
//                         }

//                         None => break,
//                     }
//                 }
//             }
//         }

//         // Xóa kết nối khi client ngắt kết nối
//         state_clone
//             .hub
//             .lock()
//             .await
//             .disconnect(user_id, connection_id);
//     });

//     Ok(response)
// }

// use actix_web::{Error, HttpRequest, HttpResponse, web};
// use actix_ws::Message;
// use futures_util::StreamExt;
// use tokio::sync::mpsc;

// use crate::services::message_service::AppState;

// pub async fn websocket(
//     req: HttpRequest,
//     body: web::Payload,
//     state: web::Data<AppState>,
//     path: web::Path<i64>,
// ) -> Result<HttpResponse, Error> {
//     // DEMO: user_id lấy từ URL.
//     // Production phải lấy từ JWT đã xác thực.
//     let user_id = path.into_inner();

//     let (response, session, mut stream) = actix_ws::handle(&req, body)?;

//     let (tx, mut rx) = mpsc::unbounded_channel::<String>();

//     let connection_id = state.hub.lock().await.connect(user_id, tx);

//     let state_clone = state.clone();

//     actix_web::rt::spawn(async move {
//         let mut session = session;

//         loop {
//             tokio::select! {

//                 incoming = stream.next() => {

//                     match incoming {

//                         Some(Ok(Message::Ping(bytes))) => {
//                             if session.pong(&bytes).await.is_err() {
//                                 break;
//                             }
//                         }

//                         Some(Ok(Message::Close(reason))) => {
//                             let _ = session.close(reason).await;
//                             break;
//                         }

//                         Some(Ok(_)) => {}

//                         Some(Err(_)) | None => break,
//                     }
//                 }

//                 outgoing = rx.recv() => {

//                     match outgoing {

//                         Some(message) => {
//                             if session.text(message).await.is_err() {
//                                 break;
//                             }
//                         }

//                         None => break,
//                     }
//                 }
//             }
//         }

//         state_clone
//             .hub
//             .lock()
//             .await
//             .disconnect(user_id, connection_id);
//     });

//     Ok(response)
// }
