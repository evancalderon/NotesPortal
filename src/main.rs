#![feature(associated_type_bounds)]
#![feature(async_closure)]
#![feature(result_option_inspect)]
#![feature(async_fn_in_trait)]
#![feature(let_chains)]

extern crate core;

mod admin_routes;
mod api_routes;
mod counter;
mod db;
mod embed_routes;
mod integration;
mod login;

use std::{collections::HashMap, fs, net::SocketAddr, path::Path, sync::Arc, time::Duration};

use axum::{extract::FromRef, middleware, routing, Router, Server};
use axum_extra::extract::cookie::Key;
use clap::Parser;
use db::{Database, UserRole};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    key: Key,
    invites: Arc<RwLock<HashMap<String, UserRole>>>,
    db: Arc<RwLock<db::DynamoDB>>,
    students: Arc<RwLock<db::CachingDynamoDBColumn<db::Student>>>,
    users: Arc<RwLock<db::CachingDynamoDBColumn<db::User>>>,
    imported: Arc<RwLock<db::CachingDynamoDBColumn<db::StudentImportedInfo>>>,
    sessions: Arc<RwLock<HashMap<String, db::Session>>>,
}

impl FromRef<AppState> for Key {
    fn from_ref(input: &AppState) -> Self {
        input.key.clone()
    }
}

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Notes Portal")]
struct AppArgs {
    #[arg(long, help = "Creates an admin invite token", default_value_t = true)]
    create_token: bool,
    #[arg(
        short,
        long,
        help = "Connects to a local DynamoDB instance at '127.0.0.1:8000'",
        default_value_t = false
    )]
    test_db: bool,
    #[arg(short, long, default_value_t = 12000)]
    port: u16,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = AppArgs::parse();

    let key_path = Path::new("session_key");
    let db = db::DynamoDB::new(args.test_db).await;
    #[allow(deprecated)]
    let students = db.column("students");
    #[allow(deprecated)]
    let users = db.column("users");
    #[allow(deprecated)]
    let imported = db.column("imported");
    let state = AppState {
        key: if key_path.exists() {
            Key::from(fs::read(key_path).unwrap().as_slice())
        } else {
            Key::generate()
        },
        invites: Arc::new(RwLock::new(HashMap::new())),
        db: Arc::new(RwLock::new(db)),
        students: Arc::new(RwLock::new(db::CachingDynamoDBColumn::from(students))),
        users: Arc::new(RwLock::new(db::CachingDynamoDBColumn::from(users))),
        imported: Arc::new(RwLock::new(db::CachingDynamoDBColumn::from(imported))),
        sessions: Arc::new(RwLock::new(HashMap::new())),
    };

    {
        let db = state.db.read().await;
        db.create_table::<db::User>("users").await;
        db.create_table::<db::Student>("students").await;
        db.create_table::<db::StudentImportedInfo>("imported").await;
    }

    if !Path::new("session_key").exists() {
        fs::write("session_key", state.key.master()).ok();
    }

    let app = Router::new()
        .nest(
            "/api",
            api_routes::routes()
                .nest("/admin", admin_routes::routes())
                .route("/role", routing::get(login::user_role_get))
                .route("/logout", routing::post(login::logout_post))
                .route_layer(middleware::from_fn_with_state(
                    state.clone(),
                    login::auth_layer_fn,
                ))
                .route("/invite", routing::post(login::install_user_post))
                .route("/login", routing::post(login::login_post)),
        )
        .merge(embed_routes::routes())
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    println!("Server started on port {}", args.port);

    if args.create_token {
        let invites = state.invites.clone();
        tokio::spawn(async move {
            let token = Uuid::new_v4().to_string();
            invites.write().await.insert(token.clone(), UserRole::Admin);

            println!(
                "Create administrator link: http://127.0.0.1:{}/create_user?token={}",
                addr.port(),
                token
            );
        });
    }

    let server = async move {
        Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(shutdown())
            .await
    };

    tokio::spawn(async move {
        let state = state.clone();
        loop {
            let db = state.db.read().await;
            db.save();
            tokio::time::sleep(Duration::from_secs(30 * 60)).await;
        }
    });

    tokio::join!(server).0.ok();
}

async fn shutdown() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    use tokio::signal::unix::{signal, SignalKind};
    #[cfg(unix)]
    let terminate = async {
        signal(SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}
