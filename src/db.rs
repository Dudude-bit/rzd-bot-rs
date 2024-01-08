use serde::{Deserialize, Serialize};
use sqlx::{Error, Executor, Sqlite, SqlitePool};
use std::collections::HashMap;
#[derive(Clone, Debug, PartialEq, PartialOrd, sqlx::Type, Deserialize, Serialize)]
#[sqlx(rename_all = "lowercase")]
enum TaskType {
    Train,
    Day,
}

pub struct RZDDatabase {
    pool: SqlitePool,
}
#[derive(Debug, sqlx::FromRow, Deserialize, Serialize)]
pub struct Task {
    id: Option<usize>,
    data: sqlx::types::Json<HashMap<String, String>>,
    _type: TaskType,
}

impl RZDDatabase {
    // async fn connect(url: &str) -> Result<Self,String>{
    //     let conn = SqlitePool::connect(url).await;
    //
    //     return match conn {
    //         Some(conn) => {
    //             Ok(RZDDatabase{ pool: conn })
    //         }
    //         Err(err) => {
    //             Err(err.into())
    //         }
    //     }
    // }

    // async fn create_task(&self, task: Task) -> Result<Task, String> {
    //     const query: &str = "INSERT INTO tasks(type, data) VALUES ($1, $2) RETURNING *;";
    //
    //     let stream = sqlx::query_as(query).bind(task._type).bind(task.data).fetch_one(&self.pool).await;
    //
    //     return Err("error".to_string())
    //
    // }
}
