use std::{
    env,
    sync::atomic::{AtomicBool, Ordering},
    time::SystemTime,
};

use futures_util::{stream::FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::{select, sync::mpsc};
use tracing::error;

static LINK: Lazy<Selector> = Lazy::new(|| Selector::parse("a[href^='/wiki/'").unwrap());
static LOG: AtomicBool = AtomicBool::new(true);

fn is_wikipedia_url(url: &str) -> bool {
    if !url.starts_with("https://") {
        return false;
    }

    let mut mobile = false;
    let mut found = false;
    // skip nationality
    let mut iter = url.split('.').enumerate().skip(1);
    loop {
        match iter.next() {
            Some((1, "m")) => {
                mobile = true;
                continue;
            }
            Some((i, "wikipedia")) if i == 1 + (mobile as usize) => {
                found = true;
            }
            Some((i, s)) if i == 2 + (mobile as usize) && s.starts_with("org/wiki/") => {
                return found;
            }
            _ => {
                return false;
            }
        }
    }
}

fn is_hitler_url(url: &str) -> bool {
    url == "/wiki/Adolf_Hitler"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let start = env::args().nth(1).expect("No starting point");
    if !is_wikipedia_url(&start) {
        eprintln!("Full wikipedia url expected");
        return;
    }

    let Some(pos) = start.find("/wiki/") else {
        eprintln!("Invalid wikipedia url");
        return;
    };
    let base_url = start[0..pos].to_owned();
    let relative_url = start[pos..].to_owned();

    let client = Client::new();
    let (tx, mut rx) = mpsc::unbounded_channel();
    tx.send(vec![relative_url]).unwrap();

    let start = SystemTime::now();
    let mut buf = FuturesUnordered::new();
    let mut already_visited = vec![];
    loop {
        select! {
            recv = rx.recv() => {
                if let Some(path) = recv {
                    let relative_url = path.last().unwrap();
                    if already_visited.contains(relative_url) {
                        continue;
                    } else {
                        already_visited.push(relative_url.clone());
                    }

                    buf.push(tokio::spawn(scrape(
                        tx.clone(),
                        client.clone(),
                        base_url.clone(),
                        path,
                    )));
                } else {
                    break;
                }
            }
            resolved = buf.next() => {
                if let Some(Ok(Err(path))) = resolved {
                    let elapsed = start.elapsed().unwrap();
                    println!("Found Hitler in {} hop:\n{path:#?}\nduration: {}s", path.len(), elapsed.as_secs());
                    // suppress shutdown logs
                    LOG.store(false, Ordering::Relaxed);
                    break;
                }
            }
        }
    }
}

async fn scrape(
    tx: mpsc::UnboundedSender<Vec<String>>,
    client: Client,
    base_url: String,
    path: Vec<String>,
) -> Result<(), Vec<String>> {
    let relative_url = path.last().unwrap();

    // skip special pages
    if relative_url.contains(':') {
        return Ok(());
    }

    if is_hitler_url(relative_url) {
        return Err(path);
    }

    let url = format!("{base_url}{relative_url}");
    let Ok(response) = client.get(url.as_str()).send().await.map_err(|e| {
        if LOG.load(Ordering::Relaxed) {
            error!("Error calling {url}: {e}");
        }
    }) else {
        return Ok(());
    };

    let Ok(text) = response.text().await.map_err(|e| {
        if LOG.load(Ordering::Relaxed) {
            error!("Error reading {url}: {e}");
        }
    }) else {
        return Ok(());
    };

    let doc = Html::parse_document(&text);
    let links = doc
        .select(&LINK)
        .flat_map(|link_el| link_el.value().attr("href"));

    for link in links {
        let _ = tx.send({
            let mut temp = path.clone();
            temp.push(link.to_owned());
            temp
        });
    }

    Ok(())
}
