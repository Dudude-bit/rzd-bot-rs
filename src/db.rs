use std::collections::HashMap;
use std::sync::Arc;

use speedb::{IteratorMode, DB};
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct RZDDb {
    inner: Mutex<DB>,
}

impl RZDDb {
    #[must_use]
    pub fn new(db: DB) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(db),
        })
    }

    pub async fn create_task(&self, data: HashMap<String, String>) -> Result<String, String> {
        let key = Uuid::new_v4().to_string();
        let data_slice = serde_json::to_vec(&data);
        if data_slice.is_err() {
            return Err(format!("cant serialize data {:?}", data_slice));
        }
        match self
            .inner
            .lock()
            .await
            .put(key.clone(), data_slice.unwrap())
        {
            Ok(_) => Ok(key),
            Err(err) => Err(err.to_string()),
        }
    }

    pub async fn delete_task_by_id(&self, task_id: String) -> Result<String, String> {
        match self.inner.lock().await.delete(task_id.clone()) {
            Ok(()) => Ok(task_id),
            Err(err) => Err(err.to_string()),
        }
    }

    // pub async fn get_task_by_id(&self, task_id: String) -> Result<HashMap<String, String>, String> {
    //
    // }

    pub async fn list_tasks(&self) -> Result<HashMap<String, HashMap<String, String>>, String> {
        let mut results: HashMap<String, HashMap<String, String>> = HashMap::new();
        for r in self
            .inner
            .lock()
            .await
            .iterator(IteratorMode::Start)
            .collect::<Vec<_>>()
        {
            match r {
                Ok(r) => {
                    let key = String::from_utf8(r.0.to_vec());
                    if key.is_err() {
                        return Err(format!("cant decode key {:?}", key));
                    }
                    let value =
                        serde_json::from_slice::<HashMap<String, String>>(r.1.to_vec().as_ref());
                    if value.is_err() {
                        return Err(format!("cant decode value {:?}", key));
                    }
                    results.insert(key.unwrap(), value.unwrap());
                }
                Err(err) => return Err(format!("cant iterate over tasks {err}")),
            }
        }

        Ok(results)
    }
}
