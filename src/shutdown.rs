use tokio::sync::watch;

#[derive(Clone)]
pub struct ShutdownSignal {
    inner: watch::Receiver<bool>,
}

impl ShutdownSignal {
    pub fn new(inner: watch::Receiver<bool>) -> Self {
        Self { inner }
    }

    pub fn clone_signal(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    pub fn is_triggered(&self) -> bool {
        *self.inner.borrow()
    }

    pub async fn wait_trigger(&mut self) {
        let _ = self.inner.changed().await;
    }
}
