use crate::counter::Counter;
use crate::db;
use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use lazy_static::lazy_static;
use reqwest::StatusCode;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(RustEmbed)]
#[folder = "embed/"]
struct EmbeddedAsset;

#[derive(Deserialize)]
struct SecretsConfigFile {
    base_url: String,
    company_id: String,
    email: String,
}

lazy_static! {
    static ref CONFIG: SecretsConfigFile = {
        serde_ini::from_str(
            &String::from_utf8(EmbeddedAsset::get("mystudio.ini").unwrap().data.to_vec())
                .unwrap()
                .to_string(),
        )
        .unwrap()
    };
}

#[derive(Serialize, Clone)]
struct GenerateStudioAttendanceTokenIn {
    company_id: String,
    email: String,
    from_page: String,
}

#[derive(Deserialize, Clone)]
struct GenerateStudioAttendanceTokenOut {
    msg: String,
}

#[derive(Serialize, Clone)]
struct AllParticipantsIn {
    company_id: String,
    email: String,
    from: String,
    from_page: String,
    program_date: String,
    token: String,
}

#[derive(Deserialize, Clone)]
struct StudentDetails {
    student_id: String,
    membership_registration_id: String,
    participant_id: String,
    participant_first_name: String,
    participant_last_name: String,
    rank_name: String,
}

#[derive(Deserialize, Clone)]
struct AllParticipantsOut {
    student_detail: HashMap<String, Vec<StudentDetails>>,
}

#[derive(Serialize, Clone)]
struct GetAvailableClassDetailsIn {
    company_id: String,
    token: String,
    email: String,
    user_login_type: String,
    from: String,
    from_page: String,
    participant_id: String,
    student_id: String,
    reg_id: String,
    reg_id_type: String,
    selected_date: String,
    student_view: String,
    #[serde(rename = "type")]
    field_type: String,
}

#[derive(Deserialize, Clone)]
struct ClassDetail {
    checkin_status: String,
    start_time: String,
}

#[derive(Deserialize, Clone)]
struct GetAvailableClassDetailsOut {
    class_details: Vec<ClassDetail>,
}

pub async fn load_students_post(State(state): State<crate::AppState>) -> Result<Response, String> {
    let db = state.db.read().await;
    let mut student_col = state.students.write().await;
    let mut imported_col = state.imported.write().await;
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let mut old_student_ids: Vec<String> = vec![];
    for student in student_col.get_values(&db).await? {
        if student.date.is_some() {
            old_student_ids.push(student.id);
        }
    }

    println!("Generating MyStudio token to grab attendance...");
    let token_out = client
        .post(format!("{}/generateStudioAttendanceToken", CONFIG.base_url))
        .json(&GenerateStudioAttendanceTokenIn {
            company_id: CONFIG.company_id.clone(),
            email: CONFIG.email.clone(),
            from_page: "attendance".to_string(),
        })
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<GenerateStudioAttendanceTokenOut>()
        .await
        .map_err(|e| e.to_string())?;

    println!("Grabbing participant data...");
    let all_participants_out = client
        .post(format!("{}/allParticipants", CONFIG.base_url))
        .json(&AllParticipantsIn {
            company_id: CONFIG.company_id.clone(),
            email: CONFIG.email.clone(),
            from: "attendance".to_string(),
            from_page: "attendance".to_string(),
            program_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            token: token_out.msg.clone(),
        })
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<AllParticipantsOut>()
        .await
        .map_err(|e| e.to_string())?;

    for (_, students) in all_participants_out.student_detail {
        for student in students {
            println!(
                "Grabbing attendance details for {} {}",
                student.participant_first_name, student.participant_last_name
            );
            let class_details = client
                .post(format!("{}/getAvailableClassDetails", CONFIG.base_url))
                .json(&GetAvailableClassDetailsIn {
                    token: token_out.msg.clone(),
                    company_id: CONFIG.company_id.clone(),
                    email: CONFIG.email.clone(),
                    user_login_type: "".to_string(),
                    from: "attendance".to_string(),
                    from_page: "attendance".to_string(),
                    participant_id: student.participant_id.clone(),
                    student_id: student.student_id.clone(),
                    reg_id: student.membership_registration_id.clone(),
                    reg_id_type: "M".to_string(),
                    selected_date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                    student_view: "Y".to_string(),
                    field_type: "membership".to_string(),
                })
                .send()
                .await
                .map_err(|e| e.to_string())?
                .json::<GetAvailableClassDetailsOut>()
                .await
                .map_err(|e| e.to_string())?;

            let mut checkin_times: Vec<String> = Vec::new();
            for class_detail in class_details.class_details {
                if class_detail.checkin_status != "" {
                    checkin_times.push(class_detail.start_time);
                }
            }

            checkin_times.sort();
            let time = checkin_times[0].trim_start_matches("0");
            let name = format!(
                "{} {}",
                student.participant_first_name, student.participant_last_name
            );

            {
                let student_info = imported_col.get(&db, &name).await;
                if let Err(err) = student_info {
                    println!("db failure: {}", err);
                    continue;
                }

                let student_info = student_info.unwrap();
                let has_info = student_info.is_some();
                fn convert_notes(notes: Vec<String>, counter: &mut Counter<u32>) -> Vec<db::Note> {
                    notes
                        .iter()
                        .map(|note| db::Note {
                            id: counter.inc(),
                            date: "".to_string(),
                            user: "".to_string(),
                            content: note.clone(),
                        })
                        .collect::<Vec<db::Note>>()
                }

                let mut logins: Option<Vec<db::Note>> = None;
                let mut notes: Option<Vec<db::Note>> = None;
                let mut behaviours: Option<Vec<db::Note>> = None;
                let mut note_id = Counter::<u32>::new();
                if let Some(info) = student_info {
                    logins = Some(convert_notes(info.logins, &mut note_id));
                    notes = Some(convert_notes(info.notes, &mut note_id));
                    behaviours = Some(convert_notes(info.behaviours, &mut note_id));
                }

                let update_student = match student_col.get(&db, &student.participant_id).await {
                    Ok(Some(val)) => {
                        let mut val = val.clone();
                        val.date = Some(chrono::Local::now());
                        val.time = Some(time.to_string());
                        val.belt = student.rank_name.replace(" Belt", "");
                        if has_info {
                            println!("Integrating missing notes for {}", name.clone());
                            if val.logins.is_empty() {
                                val.note_counter.add(logins.as_ref().unwrap().len() as u32);
                                val.logins = logins.unwrap();
                            }
                            if val.notes.is_empty() {
                                val.note_counter.add(notes.as_ref().unwrap().len() as u32);
                                val.notes = notes.unwrap();
                            }
                            if val.behaviours.is_empty() {
                                val.note_counter
                                    .add(behaviours.as_ref().unwrap().len() as u32);
                                val.behaviours = behaviours.unwrap();
                            }

                            imported_col.delete(&db, &name).await?;
                        }
                        Some(val)
                    }
                    Ok(None) => {
                        if has_info {
                            println!("Integrating notes for \"{}\"", name.clone());
                            imported_col.delete(&db, &name).await?;
                        }
                        Some(db::Student {
                            name: name.clone(),
                            id: student.participant_id.clone(),
                            date: Some(chrono::Local::now()),
                            time: Some(time.to_string()),
                            first_name: student.participant_first_name,
                            last_name: student.participant_last_name,
                            belt: student.rank_name.replace(" Belt", ""),
                            logins: logins.unwrap_or(vec![]),
                            notes: notes.unwrap_or(vec![]),
                            behaviours: behaviours.unwrap_or(vec![]),
                            assigned: None,
                            note_counter: note_id,
                        })
                    }
                    Err(err) => {
                        println!("{}", err.to_string());
                        None
                    }
                };

                if let Some(v) = update_student {
                    println!("Writing update for {}", v.name);
                    student_col.put(&db, &v.id, v.clone()).await?;
                    if let Some(pos) = old_student_ids.iter().position(|id| id.clone() == v.id) {
                        old_student_ids.swap_remove(pos);
                    }
                } else {
                    println!(
                        "An error occurred which stopped \"{}\" from being added to the database.",
                        name.to_string()
                    );
                }
            }
        }
    }

    for old_student_id in old_student_ids {
        student_col
            .get_update(&db, &old_student_id, |s| {
                s.date = None;
            })
            .await?;
    }

    Ok(StatusCode::OK.into_response())
}
