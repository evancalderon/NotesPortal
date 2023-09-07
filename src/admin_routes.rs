use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use axum::{
    extract::State,
    http::Request,
    middleware::{self, Next},
    response::IntoResponse,
    routing, Extension, Json, Router,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{self, PrimaryKeyValue};

#[derive(Deserialize)]
pub struct ChangePassIn {
    name: String,
    pass: String,
}

#[derive(Deserialize)]
pub struct DeleteUserIn {
    name: String,
}

#[derive(Serialize)]
pub struct UserOut {
    name: String,
    role: db::UserRole,
}

pub fn routes() -> Router<crate::AppState> {
    Router::new()
        .route("/clear_times", routing::post(clear_times))
        .route("/users", routing::get(users_get))
        .route("/gen_token", routing::get(gen_token_get))
        .route("/change_pass", routing::post(change_pass_post))
        .route("/delete_user", routing::post(delete_user_post))
        .route(
            "/load_students",
            routing::post(crate::integration::load_students_post),
        )
        .layer(middleware::from_fn(require_admin_layer))
}

async fn clear_times(State(state): State<crate::AppState>) -> Result<impl IntoResponse, String> {
    let db = state.db.read().await;
    let mut students = state.students.write().await;

    for student in students.get_values(&db).await? {
        if student.date.is_some() {
            students.diff_update(&db, student.get_primary_key_value().as_str(), &student, |s| {
                s.date = None;
            }).await?;
        }
    }

    Ok(())
}

async fn require_admin_layer<B>(
    Extension(session): Extension<db::Session>,
    req: Request<B>,
    next: Next<B>,
) -> impl IntoResponse {
    if session.user.role == db::UserRole::Admin {
        next.run(req).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

async fn users_get(State(state): State<crate::AppState>) -> Result<Json<Vec<UserOut>>, String> {
    let db = state.db.read().await;
    let mut users = state.users.write().await;
    Ok(Json(
        users
            .get_values(&db)
            .await?
            .iter()
            .map(|user| UserOut {
                name: user.name.clone(),
                role: user.role.clone(),
            })
            .collect(),
    ))
}

async fn gen_token_get(State(state): State<crate::AppState>) -> Json<String> {
    let mut invites = state.invites.write().await;

    let token = Uuid::new_v4();
    invites.insert(token.to_string(), db::UserRole::Standard);
    Json(token.to_string())
}

async fn change_pass_post(
    State(state): State<crate::AppState>,
    Json(payload): Json<ChangePassIn>,
) -> impl IntoResponse {
    let db = state.db.read().await;

    let mut users = state.users.write().await;
    if let Ok(Some(mut user)) = users.get(&db, &payload.name.clone()).await {
        let mut hasher = DefaultHasher::new();
        payload.pass.hash(&mut hasher);
        user.hash = hasher.finish();

        users
            .put(&db, &payload.name.to_lowercase(), user.clone())
            .await
            .ok();
    }

    StatusCode::OK
}

async fn delete_user_post(
    State(state): State<crate::AppState>,
    Json(payload): Json<DeleteUserIn>,
) -> Result<impl IntoResponse, String> {
    let db = state.db.read().await;
    let mut users = state.users.write().await;
    users.delete(&db, &payload.name.to_lowercase()).await?;

    Ok(StatusCode::OK)
}
