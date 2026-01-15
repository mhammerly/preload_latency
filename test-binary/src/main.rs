use bigtable_rs::bigtable;
use bigtable_rs::google::bigtable::v2::row_range::{EndKey, StartKey};
use bigtable_rs::google::bigtable::v2::{ReadRowsRequest, RowRange, RowSet};
use std::time::Duration;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Send a request to GitHub using system DNS
    let client = reqwest::Client::builder().hickory_dns(false).build()?;
    let resp = client.get("https://github.com/robots.txt").send().await;
    tracing::info!("{resp:#?}");

    // Send a request to bsky using hickory-dns
    let client = reqwest::Client::builder().hickory_dns(true).build()?;
    let resp = client.get("https://bsky.app/robots.txt").send().await;
    tracing::info!("{resp:#?}");

    // make a bigtable client
    let connection = bigtable::BigTableConnection::new(
        "project",
        "instance",
        true,
        4,                             /* channel_size */
        Some(Duration::from_secs(10)), /* timeout */
    )
    .await?;
    let mut bigtable = connection.client();

    // prepare a ReadRowsRequest
    let request = ReadRowsRequest {
        table_name: bigtable.get_full_table_name("table-1"),
        rows_limit: 10,
        rows: Some(RowSet {
            row_keys: vec![], // use this field to put keys for reading specific rows
            row_ranges: vec![RowRange {
                start_key: Some(StartKey::StartKeyClosed("key1".as_bytes().to_vec())),
                end_key: Some(EndKey::EndKeyOpen("key4".as_bytes().to_vec())),
            }],
        }),
        filter: None,
        ..ReadRowsRequest::default()
    };

    // calling bigtable API to get results
    let resp = bigtable.read_rows(request).await;
    tracing::info!("{resp:#?}");
    Ok(())
}
