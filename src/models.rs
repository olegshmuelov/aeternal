#![allow(proc_macro_derive_resolution_fallback)]

use super::schema::key_blocks;
use super::schema::micro_blocks;
use super::schema::transactions;

use diesel::prelude::*;
use diesel::sql_types::*;
use diesel::pg::PgConnection;

extern crate serde_json;
use serde_json::Number;

use bigdecimal;

use std;

#[derive(Queryable)]
pub struct KeyBlock {
    pub id: i32,
    pub hash: Option<String>,
    pub height: Option<i64>,
    pub miner: Option<String>,
    pub beneficiary: Option<String>,
    pub pow: Option<String>,
    pub nonce: Option<bigdecimal::BigDecimal>,
    pub prev_hash: Option<String>,
    pub prev_key_hash: Option<String>,
    pub state_hash: Option<String>,
    pub target: Option<i64>,
    pub time: Option<i64>,
    pub version: Option<i32>,
}

sql_function!(fn currval(x: VarChar) -> BigInt);

impl KeyBlock {

    pub fn max_id(conn: &PgConnection) -> Result<i32, Box<std::error::Error>> {
        let b = key_blocks::table.order(key_blocks::id.desc()).load::<KeyBlock>(conn)?;
        Ok(b.first().unwrap().id)
    }

    pub fn top_hash(conn: &PgConnection) -> Result<String, Box<std::error::Error>> {
        let b = key_blocks::table.order(key_blocks::id.desc()).load::<KeyBlock>(conn)?;
        let h = match b.first() {
            Some(x) => x,
            None => return Ok(String::new()),
        };
        Ok(h.hash.clone().unwrap().clone())
    }
                                                           
}

#[derive(Insertable)]
#[table_name="key_blocks"]
pub struct InsertableKeyBlock {
    pub hash: String,
    pub height: i64,
    pub miner: String,
    pub nonce: bigdecimal::BigDecimal,
    pub beneficiary: String,
    pub pow: String,
    pub prev_hash: String,
    pub prev_key_hash: String,
    pub state_hash: String,
    pub target: i64,
    pub time: i64,
    pub version: i32,
}


impl InsertableKeyBlock {
    
    pub fn save(&self, conn: &PgConnection) ->
        Result<i64, Box<std::error::Error>> {
            use diesel::dsl::{select, insert_into};
            use diesel::RunQueryDsl;
            use schema::key_blocks::dsl::*;
            insert_into(key_blocks)
                .values(self).execute(&*conn)?;
            let generated_id = select(currval("key_blocks_id_seq")).get_result::<i64>(&*conn)?;
            Ok(generated_id)
        }

    pub fn from_json_key_block(jb: &JsonKeyBlock) ->
        Result<InsertableKeyBlock, Box<std::error::Error>> {
            //TODO: fix this.
            let nonce: u64 = match jb.nonce.as_u64() {
                Some(val) => val,
                None => 0,
            };
            Ok(InsertableKeyBlock {
                hash: jb.hash.clone(),
                height: jb.height,
                miner: jb.miner.clone(),
                nonce: bigdecimal::BigDecimal::from(nonce),
                beneficiary: jb.beneficiary.clone(),
                pow: format!("{:?}", jb.pow),
                prev_hash: jb.prev_hash.clone(),
                prev_key_hash: jb.prev_key_hash.clone(),
                state_hash: jb.state_hash.clone(),
                target: jb.target.clone(),
                time: jb.time,
                version: jb.version,
            })
        }
}

/*
In a better world, the serialization object would be the same as we
use for persistence, but in this one right now that doesn't work,
because serde_json needs serde_json::Number, and diesel needs a
bigdecimal::BigDecimal. So this struct exists to be pulled from the
JSON.  If @newby gets smart enough he will write the implementations
for these missing methods.
*/
#[derive(Serialize, Deserialize)]
pub struct JsonKeyBlock {
    pub hash: String,
    pub height: i64,
    pub miner: String,
    pub beneficiary: String,
    #[serde(default="zero")]
    pub nonce: Number,
    #[serde(default="zero_vec_i32")]
    pub pow: Vec<i32>,
    pub prev_hash: String,
    pub prev_key_hash: String,
    pub state_hash: String,
    pub target: i64,
    pub time: i64,
    pub version: i32,
}

fn zero() -> Number {
    serde_json::Number::from_f64(0.0).unwrap()
}

fn zero_vec_i32() -> Vec<i32> {
    vec!(0)
}

#[derive(Queryable)]
pub struct MicroBlock {
    pub id: i32,
    pub key_block: i32,
    pub hash: String,
    pub pof_hash: String,
    pub prev_hash: String,
    pub prev_key_hash: String,
    pub signature: String,
    pub state_hash: String,
    pub txs_hash: String,
    pub version: i32,
}

impl MicroBlock {
    pub fn max_id(conn: &PgConnection) -> Result<i32, Box<std::error::Error>> {
        let b = micro_blocks::table.order(micro_blocks::id.desc()).load::<MicroBlock>(conn)?;
        Ok(b.first().unwrap().id)
    }
}

#[derive(Insertable)]
#[table_name="micro_blocks"]
#[derive(Serialize, Deserialize)]
pub struct InsertableMicroBlock {
    #[serde(default="zero_i32")]
    pub key_block_id: i32,
    pub hash: String,
    pub pof_hash: String,
    pub prev_hash: String,
    pub prev_key_hash: String,
    pub signature: String,
    pub state_hash: String,
    pub txs_hash: String,
    pub version: i32,
}

fn zero_i32() -> i32 {
    0
}

impl InsertableMicroBlock {
    
    pub fn save(&self, conn: &PgConnection) ->
        Result<i64, Box<std::error::Error>> {
            use diesel::dsl::{select, insert_into};
            use diesel::RunQueryDsl;
            use schema::micro_blocks::dsl::*;
            insert_into(micro_blocks)
                .values(self).execute(&*conn)?;
            let generated_id = select(currval("micro_blocks_id_seq")).get_result::<i64>(&*conn)?;
            Ok(generated_id)
        }
}

#[derive(Queryable)]
#[derive(QueryableByName)]
#[table_name="transactions"]
#[derive(Serialize, Deserialize)]
pub struct Transaction {
    pub id: i32,
    pub micro_block_id: i32,
    pub block_height: i32,
    pub block_hash: String,
    pub hash: String,
    pub signatures: String,
    pub tx: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
pub struct JsonTransaction {
    pub block_height: i32,
    pub block_hash: String,
    pub hash: String,
    pub signatures: Vec<String>,
    pub tx: serde_json::Value,
}

impl JsonTransaction {
    pub fn from_transaction(t: &Transaction) -> JsonTransaction {
        let mut signatures: Vec<String> = vec!();
        let _s = t.signatures.split(", ");
        for s in _s {
            signatures.push(String::from(s));
        }
        JsonTransaction {
            block_height: t.block_height,
            block_hash: t.block_hash.clone(),
            hash: t.hash.clone(),
            signatures: signatures,
            tx: t.tx.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct JsonTransactionList {
    pub transactions: Vec<JsonTransaction>,
}

#[derive(Insertable)]
#[table_name="transactions"]
pub struct InsertableTransaction {
    pub micro_block_id: i32,
    pub block_height: i32,
    pub block_hash: String,
    pub hash: String,
    pub signatures: String,
    pub tx_type: String,
    pub tx: serde_json::Value,
}

impl InsertableTransaction {

    pub fn save(&self, conn: &PgConnection) ->
        Result<i64, Box<std::error::Error>> {
            use diesel::dsl::{select, insert_into};
            use diesel::RunQueryDsl;
            use schema::transactions::dsl::*;
            insert_into(transactions)
                .values(self).execute(&*conn)?;
            let generated_id = select(currval("transactions_id_seq")).get_result::<i64>(&*conn)?;
            Ok(generated_id)
        }

    pub fn from_json_transaction(jt: &JsonTransaction, tx_type: String, micro_block_id: i32)
                                 -> Result<InsertableTransaction,
                                           Box<std::error::Error>>
    {
        
        let mut signatures = String::new();
        for i in 0 .. jt.signatures.len() {
            if i > 0 {
                signatures.push_str(" ");
            }
            signatures.push_str(&jt.signatures[i].clone());
        }
        Ok(InsertableTransaction {
            micro_block_id: micro_block_id,
            block_height: jt.block_height,
            block_hash: jt.block_hash.clone(),
            hash: jt.hash.clone(),
            signatures: signatures,
            tx_type: tx_type,
            tx: serde_json::from_str(&jt.tx.to_string()).unwrap(),
        })
    }
                
}
