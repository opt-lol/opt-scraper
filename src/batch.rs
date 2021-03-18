use backoff::future::retry;
use backoff::ExponentialBackoff;
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use lazy_static::lazy_static;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::io::AsyncWriteExt;

const URL: &str = "https://egov.uscis.gov/casestatus/mycasestatus.do";

/// Controls how many number validation errors we should accumulate before
/// giving up on the current batch
const NUM_VALIDATION_STOP_THRESH: usize = 100;

lazy_static! {
    static ref TITLE_SELECTOR: Selector = Selector::parse(".rows > h1:nth-child(1)").unwrap();
    static ref BODY_SELECTOR: Selector = Selector::parse(".rows > p:nth-child(2)").unwrap();
    static ref ERROR_MESSAGE_SELECTOR: Selector =
        Selector::parse("#formErrorMessages > h4:nth-child(1)").unwrap();
}

#[derive(Debug)]
enum RecordFetchErrReason {
    NumValidationError,
    // ConnectionError,
}

#[derive(Debug)]
struct RecordFetchErr {
    num: String,
    reason: RecordFetchErrReason,
}

#[derive(Debug)]
struct Record {
    id: u64,
    title: String,
    body: String,
}

impl Record {
    pub async fn fetch_record(
        client: &Client,
        service_center: &str,
        id: u64,
    ) -> Result<Self, RecordFetchErr> {
        let mut app_receipt_num = String::from(service_center);
        app_receipt_num.push_str(&id.to_string());

        let text: String = retry(ExponentialBackoff::default(), || async {
            let text = client
                .post(URL)
                .query(&[
                    ("changeLocale", ""),
                    ("completedActionsCurrentPage", "0"),
                    ("upcomingActionsCurrentPage", "0"),
                    ("appReceiptNum", &app_receipt_num),
                    ("caseStatusSearchBtn", "CHECK+STATUS"),
                ])
                .send()
                .await?
                .text()
                .await?;
            Ok(text)
        })
        .await
        .unwrap();

        let html = Html::parse_document(&text);

        match html.select(&ERROR_MESSAGE_SELECTOR).next() {
            Some(error_element) => {
                let html = error_element.inner_html();

                if html.contains("Validation Error") {
                    Err(RecordFetchErr {
                        num: app_receipt_num,
                        reason: RecordFetchErrReason::NumValidationError,
                    })
                } else {
                    panic!("Unknown record error! num: {}", app_receipt_num);
                }
            }
            None => {
                let title = html.select(&TITLE_SELECTOR).next().unwrap().inner_html();
                let body = html.select(&BODY_SELECTOR).next().unwrap().inner_html();

                Ok(Record { id, title, body })
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct Batch {
    service_center: String,
    from_id: u64,
    records: Vec<Record>,
}

impl Batch {
    pub fn new(service_center: String, from_id: u64) -> Self {
        Self {
            service_center,
            from_id,
            records: Vec::with_capacity(1000),
        }
    }

    pub async fn download(&mut self, conn_limit: usize) -> Result<(), ()> {
        println!(
            "Starting batch download: {}{}",
            self.service_center, self.from_id
        );

        let service_center = self.service_center.clone().into_boxed_str();
        let client = Client::builder().gzip(true).build().unwrap();
        let mut futs = FuturesUnordered::new();
        let mut num_validation_errors: usize = 0;

        for i in 0..1000 {
            if num_validation_errors >= NUM_VALIDATION_STOP_THRESH {
                break;
            }
            let id = self.from_id * 1000 + i;
            futs.push(Record::fetch_record(&client, &service_center, id));

            if futs.len() == conn_limit {
                let record_result = futs.next().await.unwrap();

                match record_result {
                    Ok(record) => self.records.push(record),
                    Err(err) => match err.reason {
                        RecordFetchErrReason::NumValidationError => num_validation_errors += 1,
                    },
                }
            }
        }

        while let Some(record_result) = futs.next().await {
            match record_result {
                Ok(record) => self.records.push(record),
                Err(err) => match err.reason {
                    RecordFetchErrReason::NumValidationError => (),
                },
            }
        }

        println!(
            "Finished batch download: {}{}. Records: {}",
            self.service_center,
            self.from_id,
            self.records.len()
        );

        self.records.sort_by_key(|k| k.id);

        if self.records.len() > 0 {
            Ok(())
        } else {
            Err(())
        }
    }

    pub async fn save_csv(&self, path: &std::path::PathBuf) -> Result<(), ()> {
        let mut path = path.clone();
        path.push(format!("{}{}.csv", self.service_center, self.from_id));
        println!("Saving data to {:?}", path);
        let mut file = tokio::fs::File::create(path)
            .await
            .expect("Failed to create a file");

        file.write_all(b"id,title,body\n").await.unwrap();

        for rec in &self.records {
            // id
            file.write_all(b"\"").await.unwrap();
            file.write_all(self.service_center.as_bytes())
                .await
                .unwrap();
            file.write_all(rec.id.to_string().as_bytes()).await.unwrap();
            file.write_all(b"\",").await.unwrap();

            // title
            file.write_all(b"\"").await.unwrap();
            file.write_all(rec.title.replace("\"", "\\\"").as_bytes())
                .await
                .unwrap();
            file.write_all(b"\",").await.unwrap();

            // body
            file.write_all(b"\"").await.unwrap();
            file.write_all(rec.body.replace("\"", "\"\"").as_bytes())
                .await
                .unwrap();
            file.write_all(b"\"\n").await.unwrap();
        }

        Ok(())
    }
}
