use actix_web::{HttpResponse, web};

// use crate::handlers::{group_handler, message_handler, user_handler, ws_handler};

pub fn test_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // Test router
            .route(
                "/test",
                web::get().to(|| async { HttpResponse::Ok().body("Router is working!") }),
            ), // .route("/messages", web::post().to(message_handler::create_message))
               // .route("/users", web::post().to(user_handler::create_user))
               // .route(
               //     "/{group_id}/members",
               //     web::post().to(group_handler::add_group_members),
               // ),
    );

    // cfg.route("/ws/{user_id}", web::get().to(ws_handler::websocket));
}
