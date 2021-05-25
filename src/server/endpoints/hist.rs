use crate::config::BldConfig;
use crate::persist::Database;
use crate::server::User;
use actix_web::{get, web, HttpResponse, Responder};

#[get("/hist")]
pub async fn hist((user, config): (Option<User>, web::Data<BldConfig>)) -> impl Responder {
    if user.is_none() {
        return HttpResponse::Unauthorized().body("");
    }

    match history_info(config.get_ref()) {
        Ok(ls) => HttpResponse::Ok().body(ls),
        Err(_) => HttpResponse::BadRequest().body(""),
    }
}

fn history_info(config: &BldConfig) -> anyhow::Result<String> {
    let db = Database::connect(&config.local.db)?;
    let pipelines = db.all()?;
    let info = pipelines
        .iter()
        .map(|p| p.to_string())
        .fold(String::new(), |acc, n| format!("{}\n{}\n", acc, n));
    Ok(info)
}
