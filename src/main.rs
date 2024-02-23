#[macro_use]
extern crate rocket;

use rocket::fairing::{self, AdHoc};
use rocket::{Build, Rocket};
use std::time::{SystemTime, UNIX_EPOCH};

use rocket::http::ContentType;
use rocket_db_pools::sqlx::{self, Row};
use rocket_db_pools::{Connection, Database};

#[derive(Database)]
#[database("sqlite_data")]
struct Data(sqlx::SqlitePool);

#[get("/")]
fn index() -> &'static str {
    "Hi! Try GET /data/foo or PUT /data/foo."
}

#[get("/data/<id>")]
async fn read(mut db: Connection<Data>, id: &str) -> Option<(ContentType, String)> {
    sqlx::query("SELECT payload, type FROM data WHERE id = ?")
        .bind(id)
        .fetch_one(&mut **db)
        .await
        .and_then(|row| {
            let payload: String = row.try_get(0)?;
            let content_type: String = row.try_get(1)?;
            Ok((ContentType::parse_flexible(&content_type).unwrap(), payload))
        })
        .ok()
}

#[put("/data/<id>", data = "<payload>")]
async fn write(
    mut db: Connection<Data>,
    id: &str,
    payload: &str,
    content_type: &ContentType,
) -> Option<&'static str> {
    sqlx::query("INSERT INTO data (id, type, timestamp, payload) VALUES ($1, $2, $3, $4) ON CONFLICT (id) DO UPDATE SET type = $2, timestamp = $3, payload = $4")
        .bind(id)
        .bind(content_type.to_string())
        .bind(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64)
        .bind(payload)
        .execute(&mut **db).await
        .and_then(|_| Ok("Ok"))
        .ok()
}

async fn setup(rocket: Rocket<Build>) -> fairing::Result {
    let db = Data::fetch(&rocket).expect("db");
    match sqlx::query(
        "CREATE TABLE IF NOT EXISTS data (id text UNIQUE, type text, timestamp int, payload blob)",
    )
    .execute(&**db)
    .await
    {
        Ok(_) => Ok(rocket),
        Err(e) => {
            error!("Failed to init db: {}", e);
            Err(rocket)
        }
    }
}

#[launch]
fn rocket() -> _ {
    let figment = rocket::Config::figment().merge((
        "databases.sqlite_data",
        rocket_db_pools::Config {
            url: "data.db".into(),
            min_connections: None,
            max_connections: 50,
            connect_timeout: 5,
            idle_timeout: None,
        },
    ));
    rocket::custom(figment)
        .attach(Data::init())
        .attach(AdHoc::try_on_ignite("setup db", setup))
        .mount("/", routes![index, read, write])
}
