use diesel::sqlite::SqliteConnection;
use diesel::prelude::*;
use std::env;
use std::fmt::format;

pub fn establish_connection() -> Result<SqliteConnection, String> {

    let database_url = env::var("DATABASE_URL").expect("DB_PATH must be set");
    let connection = SqliteConnection::establish(&database_url);

    return match connection {
        Ok(connection) => {
            Ok(connection)
        }
        Err(err) => {
            Err(format!("cant connect to db {err}"))
        }
    }
}