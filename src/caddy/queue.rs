use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use crate::caddy::{CaddyClient, CaddyError};

const MAX_RETRIES: u32 = 3;
const MAX_QUEUE_SIZE: usize = 500;

#[derive(Debug, Clone)]
pub enum RouteOperation {
    Add {
        domain: String,
        path: Option<String>,
        upstream: String,
    },
    Remove {
        domain: String,
    },
    Update {
        domain: String,
        upstream: String,
    },
}

struct QueuedOp {
    operation: RouteOperation,
    retries: u32,
}

pub struct RouteQueue {
    queue: Arc<Mutex<Vec<QueuedOp>>>,
    client: CaddyClient,
}

impl RouteQueue {
    pub fn new(client: CaddyClient) -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
            client,
        }
    }

    pub async fn enqueue(&self, operation: RouteOperation) {
        let mut queue = self.queue.lock().await;
        if queue.len() >= MAX_QUEUE_SIZE {
            warn!(
                "Route queue at capacity ({}), dropping oldest operation",
                MAX_QUEUE_SIZE
            );
            queue.remove(0);
        }
        queue.push(QueuedOp {
            operation,
            retries: 0,
        });
    }

    pub async fn flush(&self) -> Result<usize, CaddyError> {
        let ops: Vec<QueuedOp> = {
            let mut queue = self.queue.lock().await;
            std::mem::take(&mut *queue)
        };

        let count = ops.len();
        let mut failed = Vec::new();

        for mut queued in ops {
            let result = match &queued.operation {
                RouteOperation::Add {
                    domain,
                    path,
                    upstream,
                } => {
                    self.client
                        .add_route_with_path(domain, path.as_deref(), upstream)
                        .await
                }
                RouteOperation::Remove { domain } => self.client.remove_route(domain).await,
                RouteOperation::Update { domain, upstream } => {
                    self.client.update_route(domain, upstream).await
                }
            };

            if let Err(e) = result {
                queued.retries += 1;
                if queued.retries >= MAX_RETRIES {
                    error!(
                        "Dropping route operation after {} retries: {:?} — {e}",
                        MAX_RETRIES, queued.operation
                    );
                } else {
                    warn!(
                        "Route operation failed (retry {}/{}): {e}",
                        queued.retries, MAX_RETRIES
                    );
                    failed.push(queued);
                }
            }
        }

        if !failed.is_empty() {
            let mut queue = self.queue.lock().await;
            let retry_count = failed.len();
            for op in failed {
                queue.push(op);
            }
            warn!("{retry_count} operations re-queued for retry");
        }

        Ok(count)
    }

    pub async fn start_flush_loop(self: Arc<Self>) {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            let queue_len = self.queue.lock().await.len();
            if queue_len == 0 {
                continue;
            }

            info!("Flushing {queue_len} queued route operations");
            match self.flush().await {
                Ok(n) => info!("Flushed {n} route operations"),
                Err(e) => error!("Route flush error: {e}"),
            }
        }
    }
}
