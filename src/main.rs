mod item;
mod progress;

use futures::stream::TryStreamExt;
use tokio::io::{self, BufReader, BufWriter};
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio::sync::watch::channel;

use progress::Progress;
use crate::item::Item;

#[tokio::main]
async fn main() {
    let (tx, mut rx) = channel(Item::default());
    let (cancel_tx, cancel_rx)  = channel(false);
    let pg = Progress::new("test", tx, cancel_rx, io::sink());

    let download_handle = tokio::task::spawn(async move {
        let url = "https://releases.ubuntu.com/20.04/ubuntu-20.04-desktop-amd64.iso";
        let res = reqwest::get(url).await.unwrap();

        let stream = res
            .bytes_stream()
            .map_err(|e| futures::io::Error::new(futures::io::ErrorKind::Other, e))
            .into_async_read()
            .compat();

        let (mut reader, mut writer) = (BufReader::new(stream), BufWriter::new(pg));
        io::copy(&mut reader, &mut writer).await.unwrap();
    });

    tokio::task::spawn(async move {
        while let Some(stat) = rx.recv().await {
            println!("{:?}", stat);
            tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
        }
        println!("{:?}", rx.borrow());
    });

    tokio::task::spawn(async move {
        tokio::time::delay_for(std::time::Duration::from_millis(60*1000)).await;
        let _ = cancel_tx.broadcast(true);
    });

    download_handle.await.unwrap();
}
