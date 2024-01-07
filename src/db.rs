// use std::io::Bytes;
// use sqlx::{Error, Executor, Sqlite, SqlitePool};
// use sqlx::pool::PoolConnection;
//
// enum TaskType {
//     Train,
//     Day
// }
//
// pub struct RZDDatabase {
//     pool: SqlitePool
// }
//
// pub struct Task {
//     id: Option<usize>,
//     data: TaskType,
//     _type: String
// }
//
// impl RZDDatabase {
//     async fn connect(url: &str) -> Result<Self,String>{
//         let conn = SqlitePool::connect(url).await;
//
//         return match conn {
//             Some(conn) => {
//                 Ok(RZDDatabase{ pool: conn })
//             }
//             Err(err) => {
//                 Err(err.into())
//             }
//         }
//     }
//
//     async fn create_task(&self, task: Task) -> Result<Task, String> {
//         const query: &str = "INSERT INTO tasks(type, data) VALUES ()";
//
//         let conn = self.pool.acquire().await;
//
//         return match conn {
//             Ok(conn) => {
//                 conn.execute(query).await
//             }
//             Err(err) => {
//                 format!("cant aquire connection from pool {err}")
//             }
//         }
//     }
// }
