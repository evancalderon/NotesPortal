use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
    Extension, Json,
};
use axum_extra::extract::{Form, PrivateCookieJar};
use chrono::{Duration, Local, Utc};
use cookie::{Cookie, SameSite};
use serde::Deserialize;
use uuid::Uuid;

use crate::db;

#[derive(Deserialize)]
pub struct LoginData {
    name: String,
    pass: String,
}

#[derive(Deserialize, Clone)]
pub struct InstallUserData {
    name: String,
    pass: String,
    token: String,
}

pub async fn user_role_get(Extension(session): Extension<db::Session>) -> Json<db::UserRole> {
    Json(session.user.role)
}

pub async fn install_user_post(
    State(state): State<crate::AppState>,
    cookies: PrivateCookieJar,
    Form(form): Form<InstallUserData>,
) -> impl IntoResponse {
    let mut invites = state.invites.write().await;
    let db = state.db.read().await;
    let role = invites.remove(&form.token);
    let name = form.name.clone();

    if role.is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let mut hasher = DefaultHasher::new();
    form.pass.hash(&mut hasher);
    {
        let mut users = state.users.write().await;
        let new_user = db::User {
            name: name.clone(),
            primary_key: name.to_lowercase(),
            hash: hasher.finish(),
            role: role.clone().unwrap(),
        };
        if let Err(e) = users.put(&db, &name.to_lowercase(), new_user).await {
            return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
        }
    }

    if role.unwrap() == db::UserRole::Admin {
        return (
            login(
                &state,
                &cookies,
                &LoginData {
                    name: name.clone(),
                    pass: form.pass.clone(),
                },
            )
            .await
            .1,
            Redirect::to("/"),
        )
            .into_response();
    }

    Redirect::to("/").into_response()
}

pub async fn login_post(
    State(state): State<crate::AppState>,
    cookies: PrivateCookieJar,
    Json(payload): Json<LoginData>,
) -> impl IntoResponse {
    login(&state, &cookies, &payload).await.into_response()
}

pub async fn logout_post(
    State(state): State<crate::AppState>,
    cookies: PrivateCookieJar,
) -> impl IntoResponse {
    let mut sessions = state.sessions.write().await;
    if let Some(token) = cookies.get("token") {
        sessions.remove(token.value());
        (StatusCode::OK, cookies.remove(token)).into_response()
    } else {
        StatusCode::OK.into_response()
    }
}

pub async fn auth_layer_fn<B>(
    State(state): State<crate::AppState>,
    cookies: PrivateCookieJar,
    mut req: Request<B>,
    next: Next<B>,
) -> Response {
    if let Some(token) = cookies.get("token") {
        match get_session(token.value(), &state).await {
            Ok(Some(session)) => {
                req.extensions_mut().insert(session);
                next.run(req).await
            }
            Ok(None) => (StatusCode::UNAUTHORIZED, cookies.remove(token)).into_response(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
        }
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

async fn login(
    state: &crate::AppState,
    cookies: &PrivateCookieJar,
    data: &LoginData,
) -> (StatusCode, PrivateCookieJar) {
    let db = state.db.read().await;
    let mut users = state.users.write().await;
    let mut sessions = state.sessions.write().await;

    let mut hasher = DefaultHasher::new();
    data.pass.hash(&mut hasher);

    let user = users.get(&db, &data.name.to_lowercase()).await;
    if let Ok(Some(user)) = user && user.hash == hasher.finish() {
        let token = Uuid::new_v4().to_string();
        let expires = Utc::now()
            .checked_add_signed(Duration::hours(4))
            .unwrap()
            .timestamp();
        sessions.insert(
            token.clone(),
            db::Session {
                token: token.clone(),
                expires,
                user: user.clone(),
            },
        );
        let cookie = Cookie::build("token", token)
            .max_age(cookie::time::Duration::hours(4))
            .same_site(SameSite::Lax)
            .path("/")
            .finish();
        let mut cookies = cookies.clone();
        if let Some(cookie) = cookies.get("token") {
            cookies = cookies.remove(cookie);
        }
        cookies = cookies.add(cookie);
        (StatusCode::OK, cookies)
    } else {
        (StatusCode::UNAUTHORIZED, cookies.clone())
    }
}

async fn get_session(token: &str, state: &crate::AppState) -> Result<Option<db::Session>, String> {
    let mut sessions = state.sessions.write().await;

    if let Some(session) = sessions.get(token.clone()) {
        if session.expires >= Local::now().timestamp() {
            Ok(Some(session.clone()))
        } else {
            sessions.remove(token.clone());
            Ok(None)
        }
    } else {
        Ok(None)
    }
}
