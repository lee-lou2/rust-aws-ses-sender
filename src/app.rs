use crate::handlers;
use crate::middlewares;
use crate::state;
use axum::routing::delete;
use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

pub async fn app(state: state::AppState) -> Result<Router, sqlx::Error> {
    // Configure router
    let app = Router::new()
        // Messages
        .route(
            "/v1/messages",
            post(handlers::message_handlers::create_message_handler)
                .layer(from_fn(middlewares::auth_middlewares::jwt_auth_middleware)),
        )
        // Topics
        .route(
            "/v1/topics/{topic_id}",
            get(handlers::topic_handlers::retrieve_topic_handler)
                .layer(from_fn(middlewares::auth_middlewares::jwt_auth_middleware)),
        )
        .route(
            "/v1/topics/{topic_id}",
            delete(handlers::topic_handlers::stop_topic_handler)
                .layer(from_fn(middlewares::auth_middlewares::jwt_auth_middleware)),
        )
        // Events
        .route(
            "/v1/events/open",
            get(handlers::event_handlers::open_message_handler),
        )
        .route(
            "/v1/events/counts/sent",
            get(handlers::event_handlers::get_sent_count_handler)
                .layer(from_fn(middlewares::auth_middlewares::jwt_auth_middleware)),
        )
        .route(
            "/v1/events/results",
            post(handlers::event_handlers::create_event_handler),
        )
        .with_state(state)
        .layer(TraceLayer::new_for_http());
    Ok(app)
}
