#[macro_use]
extern crate rocket;

use mongodb::bson::doc;
use mongodb::{Client, Collection};
use rand::distr::Alphanumeric;
use rand::Rng;
use rocket::http::{Method, Status};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::State;
use rocket_cors::{AllowedOrigins, CorsOptions};
use rocket_okapi::openapi;
use rocket_okapi::{openapi_get_routes, swagger_ui::make_swagger_ui};
use std::env;

mod models;
use models::Url;

struct MongoDb {
    urls: Collection<Url>,
}

const SHORT_ID_LENGTH: usize = 6;
const DB_NAME: &str = "url_shortener";
const COLLECTION_NAME: &str = "urls";

/// generate random 6-char string
fn generate_short_id() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(SHORT_ID_LENGTH)
        .map(char::from)
        .collect()
}

/// Create a shortened URL for the given long URL.
#[openapi]
#[post("/shorten", data = "<long_url>")]
async fn create_short_url(long_url: String, db: &State<MongoDb>) -> Result<Json<Url>, Status> {
    let short_id = generate_short_id();
    let url = Url {
        short_id: short_id.clone(),
        long_url,
    };

    match db.urls.insert_one(&url).await {
        Ok(_) => Ok(Json(url)),
        Err(_) => Err(Status::InternalServerError),
    }
}

/// Redirect to the long URL that corresponds to the provided short identifier.
#[openapi]
#[get("/<short_id>")]
async fn redirect_url(short_id: String, db: &State<MongoDb>) -> Result<Redirect, Status> {
    match db
        .urls
        .find_one(mongodb::bson::doc! { "short_id": &short_id })
        .await
    {
        Ok(Some(url)) => Ok(Redirect::to(url.long_url)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    // load env variables
    dotenv::dotenv().ok();

    // setup cors
    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::all())
        .allowed_methods(
            vec![Method::Get, Method::Post, Method::Patch, Method::Delete]
                .into_iter()
                .map(From::from)
                .collect(),
        )
        .allow_credentials(true);

    // Connect to mongodb
    let mongo_uri = env::var("MONGO_URI")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("DATABASE_URL or MONGO_URI must be set");

    let client = Client::with_uri_str(&mongo_uri)
        .await
        .expect("Failed to connect to MongoDB");
    let db = client.database(DB_NAME);
    let urls_collection = db.collection(COLLECTION_NAME);

    let mongodb_state = MongoDb {
        urls: urls_collection,
    };

    let swagger_config = rocket_okapi::swagger_ui::SwaggerUIConfig {
        url: "/openapi.json".to_owned(),
        ..Default::default()
    };

    rocket::build()
        .mount("/", openapi_get_routes![create_short_url, redirect_url])
        .mount("/docs", make_swagger_ui(&swagger_config))
        .manage(mongodb_state)
        .manage(cors)
        .launch()
        .await?;

    Ok(())
}
