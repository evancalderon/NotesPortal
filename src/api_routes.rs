use std::io::Read;
use std::{
    collections::{hash_map::DefaultHasher, HashMap},
    hash::{Hash, Hasher},
};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing, Extension, Json, Router,
};
use chrono::{Datelike, Duration, Local};

use crate::counter::Counter;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::db;

#[derive(Deserialize)]
struct ChangePassIn {
    new_pass: String,
}

#[derive(Deserialize)]
struct StudentNotePut {
    note: String,
}

#[derive(Deserialize)]
struct StudentNotePatch {
    id: u32,
    note: String,
}

pub fn routes() -> Router<crate::AppState> {
    Router::new()
        .route("/change_pass", routing::post(change_pass_post))
        .route(
            "/students/:id/:note_type",
            routing::put(student_note_put)
                .delete(student_note_delete)
                .patch(student_note_patch),
        )
        .route("/students", routing::get(students_get))
        .route("/senseis", routing::get(senseis_get))
        .route("/load_csv", routing::post(load_csv_post))
}

async fn change_pass_post(
    State(state): State<crate::AppState>,
    Extension(session): Extension<db::Session>,
    Json(payload): Json<ChangePassIn>,
) -> impl IntoResponse {
    let db = state.db.read().await;

    let mut users = state.users.write().await;
    if let Ok(Some(mut user)) = users.get(&db, &session.user.name.clone()).await {
        let original_user = user.clone();
        let mut hasher = DefaultHasher::new();
        payload.new_pass.hash(&mut hasher);
        user.hash = hasher.finish();

        users
            .diff_update(&db, &user.name, &original_user, |u| u.hash = user.hash)
            .await
            .ok();
    }

    StatusCode::OK
}

async fn student_note_put(
    Extension(session): Extension<db::Session>,
    Path((id, note_type)): Path<(String, String)>,
    State(state): State<crate::AppState>,
    Json(payload): Json<StudentNotePut>,
) -> Result<impl IntoResponse, String> {
    let db = state.db.read().await;
    if !["logins", "notes", "behaviours", "assigned"].contains(&note_type.as_str()) {
        return Ok((
            StatusCode::BAD_REQUEST,
            "`note_type` must be one of ['logins', 'notes', 'behaviours', 'assigned']",
        ));
    }
    let mut students = state.students.write().await;
    let original_student = students.get(&db, id.as_str()).await;
    if let Ok(Some(original_student)) = original_student {
        let result = students
            .diff_update(&db, &id.clone(), &original_student, |student| {
                let date = chrono::Local::now().format("%m-%d-%y").to_string();
                match note_type.as_str() {
                    "logins" => student.logins.push(db::Note {
                        id: student.note_counter.inc(),
                        date,
                        user: session.user.name.clone(),
                        content: payload.note.clone(),
                    }),
                    "notes" => student.notes.push(db::Note {
                        id: student.note_counter.inc(),
                        date,
                        user: session.user.name.clone(),
                        content: payload.note.clone(),
                    }),
                    "behaviours" => student.behaviours.push(db::Note {
                        id: student.note_counter.inc(),
                        date,
                        user: session.user.name.clone(),
                        content: payload.note.clone(),
                    }),
                    "assigned" => student.assigned = Some(payload.note.clone()),
                    _ => {}
                }
            })
            .await;

        if result.is_err() {
            Ok((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update note."))
        } else {
            Ok((StatusCode::OK, ""))
        }
    } else {
        Ok((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update note."))
    }
}

#[allow(unused)]
async fn student_note_patch(
    Path((id, note_type)): Path<(String, String)>,
    State(state): State<crate::AppState>,
    Json(payload): Json<StudentNotePatch>,
) -> Result<impl IntoResponse, String> {
    let db = state.db.read().await;

    if !["logins", "notes", "behaviours"].contains(&note_type.as_str()) {
        return Ok((
            StatusCode::BAD_REQUEST,
            "`note_type` must be one of ['logins', 'notes', 'behaviours']",
        ));
    }
    let mut students = state.students.write().await;
    let original_student = students.get(&db, id.as_str()).await;
    if let Ok(Some(original_student)) = original_student {
        let result = students
            .diff_update(&db, &id.clone(), &original_student, |student| {
                macro_rules! edit_note {
                    ($b:ident) => {{
                        if let Some(idx) = student.$b.iter().position(|v| v.id == payload.id) {
                            if let Some(v) = student.logins.get_mut(idx) {
                                v.content = payload.note;
                            }
                        }
                    }};
                }

                match note_type.as_str() {
                    "logins" => edit_note!(logins),
                    "notes" => edit_note!(notes),
                    "behaviours" => edit_note!(behaviours),
                    _ => {}
                }
            })
            .await;

        if result.is_err() {
            Ok((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update note."))
        } else {
            Ok((StatusCode::OK, ""))
        }
    } else {
        Ok((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update note."))
    }
}

async fn student_note_delete(
    Path((id, note_type)): Path<(String, String)>,
    State(state): State<crate::AppState>,
    Json(note_id): Json<u32>,
) -> Result<impl IntoResponse, String> {
    let db = state.db.read().await;

    if !["logins", "notes", "behaviours", "assigned"].contains(&note_type.as_str()) {
        return Ok((
            StatusCode::BAD_REQUEST,
            "`note_type` needs to be one of ['logins', 'notes', 'behaviours', 'assigned']",
        ));
    }
    let mut students = state.students.write().await;
    let original_student = students.get(&db, id.as_str()).await;
    if let Ok(Some(original_student)) = original_student {
        if students
            .diff_update(&db, &id.clone(), &original_student, |student| {
                macro_rules! remove_val {
                    ($b:ident, $v:expr) => {{
                        if let Some(index) = student.$b.iter().position(|v| v.to_owned().id == $v) {
                            student.$b.remove(index);
                        }
                    }};
                }

                match note_type.as_str() {
                    "logins" => remove_val!(logins, note_id),
                    "notes" => remove_val!(notes, note_id),
                    "behaviours" => remove_val!(behaviours, note_id),
                    "assigned" => {
                        student.assigned = None;
                    }
                    _ => {}
                }
            })
            .await
            .is_err()
        {
            Ok((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update note"))
        } else {
            Ok((StatusCode::OK, ""))
        }
    } else {
        Ok((StatusCode::INTERNAL_SERVER_ERROR, "Failed to update note"))
    }
}

async fn senseis_get(State(state): State<crate::AppState>) -> Result<Json<Vec<String>>, String> {
    let db = state.db.read().await;
    let mut users = state.users.write().await;
    Ok(Json(
        users
            .get_values(&db)
            .await?
            .iter()
            .map(|v| v.name.clone())
            .collect(),
    ))
}

async fn students_get(
    State(state): State<crate::AppState>,
) -> Result<Json<Vec<db::Student>>, String> {
    let db = state.db.read().await;
    let mut students = state.students.write().await;
    let mut imported = state.imported.write().await;

    let mut students_map: HashMap<String, db::Student> = HashMap::new();
    students
        .get_values(&db)
        .await?
        .iter()
        .cloned()
        .for_each(|mut s| {
            if s.date.is_some_and(|d| {
                Local::now() - d >= Duration::days(1) || d.day() != Local::now().day()
            }) {
                s.date = None;
            }
            students_map.insert(s.name.to_lowercase(), s);
        });
    imported
        .get_values(&db)
        .await?
        .iter()
        .cloned()
        .map(|v| {
            let mut note_id = 0u32;
            let logins = v
                .logins
                .iter()
                .map(|note| {
                    note_id += 1;
                    db::Note {
                        id: note_id,
                        date: "".to_string(),
                        user: "".to_string(),
                        content: note.clone(),
                    }
                })
                .collect::<Vec<db::Note>>();
            let notes = v
                .notes
                .iter()
                .map(|note| {
                    note_id += 1;
                    db::Note {
                        id: note_id,
                        date: "".to_string(),
                        user: "".to_string(),
                        content: note.clone(),
                    }
                })
                .collect::<Vec<db::Note>>();
            let behaviours = v
                .behaviours
                .iter()
                .map(|note| {
                    note_id += 1;
                    db::Note {
                        id: note_id,
                        date: "".to_string(),
                        user: "".to_string(),
                        content: note.clone(),
                    }
                })
                .collect::<Vec<db::Note>>();
            let (first_name, last_name) = v.name.split_at(v.name.find(" ").unwrap_or(0));
            db::Student {
                first_name: first_name.to_string(),
                last_name: last_name.to_string(),
                id: "".to_string(),
                name: v.name.to_string(),
                date: None,
                time: None,
                belt: v.belt,
                logins,
                notes,
                behaviours,
                assigned: None,
                note_counter: Counter::from(0),
            }
        })
        .for_each(|s| {
            if !students_map.contains_key(&s.name.to_lowercase()) {
                students_map.insert(s.name.to_lowercase(), s);
            }
        });

    Ok(Json(students_map.values().cloned().collect()))
}

async fn load_csv_post(
    State(state): State<crate::AppState>,
    mut files: axum::extract::Multipart,
) -> Result<(), String> {
    let db = state.db.read().await;
    let mut imported = state.imported.write().await;

    let mut row_data = None;
    while let Some(field) = files.next_field().await.unwrap_or(None) {
        let result = match field.name().unwrap_or("") {
            "row_data" => {
                let mut row_str = String::new();
                field
                    .bytes()
                    .await
                    .map_err(|e| e.to_string())?
                    .iter()
                    .as_slice()
                    .read_to_string(&mut row_str)
                    .ok();
                let json = serde_json::from_str::<Vec<u8>>(row_str.as_str());
                if let Ok(v) = json {
                    row_data = Some(v);
                    Ok(())
                } else {
                    Err(json.unwrap_err().to_string())
                }
            }
            "file" if row_data.is_some() => {
                let bytes = field.bytes().await.unwrap();
                let slice = bytes.iter().as_slice();
                let mut reader = csv::ReaderBuilder::new()
                    .has_headers(true)
                    .flexible(true)
                    .from_reader(slice);
                let mut iter = reader.records();
                let mut student_info: Vec<db::StudentImportedInfo> = Vec::new();
                let row_data = row_data.clone().unwrap();
                while let Some(row) = iter.next() {
                    if let Ok(row) = row {
                        let imported_info = db::StudentImportedInfo {
                            name: row
                                .get(*row_data.get(0).unwrap() as usize - 1)
                                .unwrap_or("")
                                .to_string(),
                            belt: row
                                .get(*row_data.get(1).unwrap() as usize - 1)
                                .unwrap_or("")
                                .to_string(),
                            logins: row
                                .get(*row_data.get(2).unwrap() as usize - 1)
                                .unwrap_or("")
                                .split("\r\n")
                                .filter(|v| *v != "")
                                .map(|v| v.to_string())
                                .collect(),
                            notes: row
                                .get(*row_data.get(3).unwrap() as usize - 1)
                                .unwrap_or("")
                                .split("\r\n")
                                .filter(|v| *v != "")
                                .map(|v| v.to_string())
                                .collect(),
                            behaviours: row
                                .get(*row_data.get(4).unwrap() as usize - 1)
                                .unwrap_or("")
                                .split("\r\n")
                                .filter(|v| *v != "")
                                .map(|v| v.to_string())
                                .collect(),
                        };
                        student_info.push(imported_info);
                    } else {
                        println!("{}", row.unwrap_err());
                    }
                }

                for student_info in student_info {
                    if student_info.name == "" {
                        continue;
                    }

                    imported
                        .put(&db, &student_info.name.to_lowercase(), student_info.clone())
                        .await
                        .ok();
                }

                Ok(())
            }
            "file" if row_data.is_none() => Err("No row data".to_string()),
            _ => Err("Invalid row data".to_string()),
        };

        if result.is_err() {
            return Err(result.unwrap_err().to_string());
        }
    }

    Ok(())
}
