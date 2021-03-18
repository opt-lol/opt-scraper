const CONNECTION_LIMIT: usize = 50;

#[derive(Debug)]
pub(crate) struct App {
    service_center: String,
    from_id: u64,
    to_id: Option<u64>,
    save_path: std::path::PathBuf,
}

impl App {
    pub fn new(
        service_center: String,
        from_id: u64,
        to_id: Option<u64>,
        save_path: std::path::PathBuf,
    ) -> Self {
        Self {
            service_center,
            from_id,
            to_id,
            save_path,
        }
    }

    pub async fn run(&self) {
        match self.to_id {
            Some(_) => self.run_to_id().await,
            None => self.run_with_auto_stop().await,
        }
    }

    async fn run_to_id(&self) {
        for id in self.from_id..self.to_id.unwrap() {
            let mut batch = crate::Batch::new(self.service_center.clone(), id);
            batch.download(CONNECTION_LIMIT).await.ok(); // ignoring the error here for now
            batch.save_csv(&self.save_path).await.unwrap();
        }
    }

    async fn run_with_auto_stop(&self) {
        let mut id = self.from_id;
        loop {
            let mut batch = crate::Batch::new(self.service_center.clone(), id);
            match batch.download(CONNECTION_LIMIT).await {
                Ok(_) => {
                    batch.save_csv(&self.save_path).await.unwrap();
                    id += 1;
                }
                Err(_) => {
                    break;
                }
            }
        }
    }
}
