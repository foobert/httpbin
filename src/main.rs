#[macro_use] extern crate rocket;

use std::time::{SystemTime, UNIX_EPOCH};

use rocket::http::ContentType;
use rocket_db_pools::{Database, Connection};
use rocket_db_pools::sqlx::{self, Row};

#[derive(Database)]
#[database("sqlite_data")]
struct Data(sqlx::SqlitePool);

#[get("/")]
fn index() -> &'static str {
    "Hi! Try GET /data/foo or PUT /data/foo."
}

#[get("/data/<id>")]
async fn read(mut db: Connection<Data>, id: &str) -> Option<(ContentType, String)> {
   sqlx::query("SELECT payload, type FROM data WHERE id = ?").bind(id)
       .fetch_one(&mut **db).await
       .and_then(|row| {
           let payload: String = row.try_get(0)?;
           let content_type: String = row.try_get(1)?;
           Ok((ContentType::parse_flexible(&content_type).unwrap(), payload))
       })
       .ok()
}

#[put("/data/<id>", data = "<payload>")]
async fn write(mut db: Connection<Data>, id: &str, payload: &str, content_type: &ContentType) -> Option<&'static str> {
    sqlx::query("INSERT INTO data (id, type, timestamp, payload) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET type = $2, timestamp = $3, payload = $4")
        .bind(id)
        .bind(content_type.to_string())
        .bind(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64)
        .bind(payload)
        .execute(&mut **db).await
        .and_then(|_| Ok("Ok"))
        .ok()
}

#[launch]
fn rocket() -> _ {
    rocket::build().attach(Data::init()).mount("/", routes![index, read, write])
}
