use diesel::sql_query;

use epoch;
use epoch::Epoch;
use models::*;

use diesel::pg::PgConnection;
use diesel::RunQueryDsl;
use r2d2::Pool;
use r2d2_diesel::ConnectionManager;
use rocket;
use rocket::response::Failure;
use rocket::http::{Method, Status};
use rocket::Outcome::{Success};
use rocket::{Outcome, State};
use rocket_contrib::Json;
use rocket_cors;
use rocket_cors::{AllowedHeaders, AllowedOrigins};
use serde_json;
use std::path::PathBuf;
use std::sync::Arc;

pub struct MiddlewareServer {
    pub epoch: Epoch,
    pub dest_url: String, // address to forward to
    pub port: u16,        // port to listen on
    pub connection: Arc<Pool<ConnectionManager<PgConnection>>>, // DB connection
}

// SQL santitizing method to prevent injection attacks.
fn sanitize(s: String) -> String {
    s.replace("'", "\\'")
}

/*
 * GET handler for Epoch
 */
#[get("/<path..>", rank=6)]
fn epoch_get_handler(state: State<MiddlewareServer>, path: PathBuf) -> Json {
    Json(
        state
            .epoch
            .get_naked(&String::from("/v2/"), &String::from(path.to_str().unwrap()))
            .unwrap(),
    )
}

#[get("/v2/<path..>")]
fn epoch_test_handler(state: State<MiddlewareServer>, path: PathBuf) -> Json<serde_json::Value> {
    Json(
        state
            .epoch
            .get_naked(&String::from("/v2/"), &String::from(path.to_str().unwrap()))
            .unwrap(),
    )
}

/*
 * POST handler for Epoch
 */
#[post("/<path..>", format = "application/json", data = "<body>")]
fn epoch_post_handler(state: State<MiddlewareServer>, path: PathBuf, body: String) -> Json {
    debug!("{}", body);
    let response = state
        .epoch
        .post_naked(
            &String::from("/v2/"),
            &String::from(path.to_str().unwrap()),
            body,
        )
        .unwrap();
    debug!("Response: {}", response);
    Json(serde_json::from_str(response.as_str()).unwrap())
}

/*
 * Epoch's only endpoint which lives outside of /v2/...
 */
#[get("/",)]
fn epoch_api_handler(state: State<MiddlewareServer>) -> Json<serde_json::Value> {
    Json(
        state
            .epoch
            .get_naked(&String::from("/api"), &String::from(""))
            .unwrap(),
    )
}

#[get("/generations/height/<height>", rank=1)]
fn generation_at_height(state: State<MiddlewareServer>, height: i64) -> Json {
    let conn = epoch::establish_connection().get().unwrap();
    let key_block = match KeyBlock::load_at_height(&conn, height) {
        Some(x) => x,
        None => {
            info!("Generation ot found at height {}", height);
            let mut path = std::path::PathBuf::new();
            path.push(format!("/generations/height/{}", height));
            return epoch_get_handler(state, path);
        }
    };
    info!("Serving generation {} from DB", height);
    let sql = format!("SELECT hash FROM micro_blocks WHERE key_block_id={}", key_block.id);
    debug!("{}", &sql);
    let mut micro_block_hashes = Vec::new();
    for row in &epoch::establish_sql_connection().query(&sql, &[]).unwrap() {
        micro_block_hashes.push(row.get(0));
    }
    Json(serde_json::from_str(&serde_json::to_string(&JsonGeneration {
        key_block: JsonKeyBlock::from_key_block(&key_block),
        micro_blocks: micro_block_hashes,
    }).unwrap()).unwrap())
}

/*
 * Gets all transactions for an account
 */
#[get("/transactions/account/<account>")]
fn transactions_for_account(state: State<MiddlewareServer>, account: String) -> Json<JsonTransactionList> {
    let sql = format!("select * from transactions where tx->>'sender_id'='{}' order by id asc", sanitize(account));
    let transactions: Vec<Transaction> = sql_query(sql).load(&*state.connection.get().unwrap()).unwrap();
    let mut trans: Vec<JsonTransaction> = vec!();
    for i in 0 .. transactions.len() {
        trans.push(JsonTransaction::from_transaction(&transactions[i]));
    }
    let list = JsonTransactionList {
        transactions: trans,
    };
    Json(list)
}

/*
 * Gets transactions between blocks
 */
#[get("/transactions/interval/<from>/<to>")]
fn transactions_for_interval(state: State<MiddlewareServer>, from: i64, to: i64) ->
    Json<JsonTransactionList> {
    let sql = format!("select t.* from transactions t, micro_blocks m, key_blocks k where t.micro_block_id=m.id and m.key_block_id=k.id and k.height >={} and k.height <= {} order by k.height asc", from, to);
    let transactions: Vec<Transaction> = sql_query(sql).load(&*state.connection.get().unwrap()).unwrap();
    let mut trans: Vec<JsonTransaction> = vec!();
    for i in 0 .. transactions.len() {
        trans.push(JsonTransaction::from_transaction(&transactions[i]));
    }
    let list = JsonTransactionList {
        transactions: trans,
    };
    Json(list)
}

/*
 * Gets average gas price for a block
 */
#[get("/key-blocks/height/<height>/gas-price")]
fn key_block_gas_price(state: State<MiddlewareServer>, height: i64) -> Option<String> {
    let sql = format!("\
select t.* from transactions t, micro_blocks m, key_blocks k where \
t.micro_block_id=m.id and \
m.key_block_id=k.id and \
k.height = {} and \
t.tx_type in ('SpendTx')", height);
    println!("{}", sql);
    let transactions: Vec<Transaction> = sql_query(sql).load(&*state.connection.get().unwrap()).unwrap();
    let mut fees: i64 = 0;
    let mut sizes: i64 = 0;
    for i in 0 .. transactions.len() {
        fees += transactions[i].fee;
        sizes += transactions[i].size as i64;
    }
    if sizes == 0 {
        return None;
    }
    Some(format!("{}", fees/sizes as i64))
}


impl MiddlewareServer {
    pub fn start(self) {
        let allowed_origins = AllowedOrigins::all();
        let options = rocket_cors::Cors {
            allowed_origins,
            allowed_methods: vec![Method::Get].into_iter().map(From::from).collect(),
            allowed_headers: AllowedHeaders::some(&["Authorization", "Accept"]),
            allow_credentials: true,
            ..Default::default()
        };

        rocket::ignite()
            .mount("/middleware", routes![transactions_for_account])
            .mount("/middleware", routes![transactions_for_interval])
            .mount("/middleware", routes![key_block_gas_price])
            .mount("/v2", routes![epoch_get_handler])
            .mount("/v2", routes![epoch_post_handler])
            .mount("/api", routes![epoch_api_handler])
            .mount("/v2", routes![generation_at_height])
            .attach(options)
            .manage(self)
            .launch();
    }
}
