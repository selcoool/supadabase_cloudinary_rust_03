use actix_web::web;

use crate::handlers::{group_handler, message_handler, user_handler, ws_handler};

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // =====================================
    // USERS
    // =====================================

    cfg.service(
        web::scope("/api/users")
            .route("", web::post().to(user_handler::create_user))
            .route("", web::get().to(user_handler::get_all_users))
            .route("/{id}", web::get().to(user_handler::get_user))
            .route("/{id}", web::delete().to(user_handler::delete_user))
            .route("/{id}", web::put().to(user_handler::update_user)),
    );

    // =====================================
    // MESSAGES
    // =====================================

    cfg.service(
        web::scope("/api/messages").route("", web::post().to(message_handler::create_message)),
    );

    // =====================================
    // GROUPS
    // =====================================

    cfg.service(
        web::scope("/api/groups")
            .route("", web::post().to(group_handler::create_group))
            .route(
                "/{group_id}/members",
                web::post().to(group_handler::add_group_members),
            ),
    );

    // =====================================
    // WEBSOCKET
    // =====================================

    cfg.route("/ws/{user_id}", web::get().to(ws_handler::websocket));
}

// // use crate::handlers::user_handler;

// use crate::handlers::{group_handler, message_handler, user_handler, ws_handler};
// use actix_web::web;
// pub fn configure_routes(cfg: &mut web::ServiceConfig) {
//     cfg.service(
//         web::scope("/api/users")
//             .route("", web::post().to(user_handler::create_user))
//             .route("", web::get().to(user_handler::get_all_users))
//             .route("/{id}", web::get().to(user_handler::get_user))
//             .route("/{id}", web::delete().to(user_handler::delete_user))
//             .route("/{id}", web::put().to(user_handler::update_user))
//             .route("/messages", web::post().to(message_handler::create_message))
//             .route("/create/group", web::post().to(group_handler::create_group))
//             .route(
//                 "/{group_id}/members",
//                 web::post().to(group_handler::add_group_members),
//             ),
//     );

//     cfg.route("/ws/{user_id}", web::get().to(ws_handler::websocket));
// }
