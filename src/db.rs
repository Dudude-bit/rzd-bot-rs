use speedb::DB;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
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

    // pub async fn create_task(&self, task_type: String, data: HashMap<String, String>) -> Result<String, String> {
    //     self.inner.lock().await.iterator()
    // }
}
