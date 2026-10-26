use crate::handlers::user_handler;
use actix_web::web;
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/users")
            .route("", web::post().to(user_handler::create_user))
            .route("", web::get().to(user_handler::get_all_users))
            .route("/{id}", web::get().to(user_handler::get_user))
            .route("/{id}", web::delete().to(user_handler::delete_user))
            .route("/{id}", web::put().to(user_handler::update_user)),
    );
}
