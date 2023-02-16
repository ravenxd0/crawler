use rayon::prelude::*;
use reqwest::blocking::Client;
use reqwest::Url;
use select::document::Document;
use select::predicate::Name;
use std::collections::HashSet;
use std::env;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::time::Instant;

struct Crawler {
    client: Client,
    given_url: String,
    found_urls: HashSet<String>,
}

impl Crawler {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let crawler = Self {
            client: Client::builder()
                .danger_accept_invalid_certs(true)
                .build()?,
            given_url: String::new(),
            found_urls: HashSet::new(),
        };

        Ok(crawler)
    }

    pub fn crawl(&mut self, url: &str) -> Result<(), Box<dyn Error>> {
        let now = Instant::now();

        // HashSet to keep track of visited links
        let mut visited = HashSet::new();
        visited.insert(url.to_string());

        // Given URL
        self.given_url = url.to_string();

        let body = self.fetch_html(url); // HTML
        let found_urls = self.get_links(&body); // Links found in html

        let mut new_urls: HashSet<String> = found_urls
            .difference(&visited) // remove visited urls from found urls
            .map(std::string::ToString::to_string)
            .collect();

        while !new_urls.is_empty() {
            let found_urls: HashSet<String> = new_urls
                .par_iter()
                .map(|url| {
                    let body = self.fetch_html(url);
                    let links = self.get_links(&body);
                    println!("Visited: {url} found {} links", links.len());
                    links
                })
                .reduce(HashSet::new, |mut acc, x| {
                    acc.extend(x);
                    acc
                });
            visited.extend(new_urls.clone()); // Add visited links

            // Remove visited links from founded links
            new_urls = found_urls
                .difference(&visited)
                .map(std::string::ToString::to_string)
                .collect();

            println!("New Urls: {}", new_urls.len());
        }
        println!("Total links visited : {}", visited.len());
        println!("Time Elapsed: {} seconds", now.elapsed().as_secs());
        self.found_urls = visited;
        self.save_urls()?;

        Ok(())
    }

    fn fetch_html(&self, url: &str) -> String {
        let mut res = self
            .client
            .get(url)
            .send()
            .expect("Error fetching URL: {url}");
        println!("Status for {}: {}", url, res.status());

        let mut body = String::new();
        res.read_to_string(&mut body).unwrap();
        body
    }

    fn get_links(&self, html: &str) -> HashSet<String> {
        let given_url = self.given_url.as_str();
        Document::from(html)
            .find(Name("a"))
            .filter_map(|n| n.attr("href"))
            .filter(|url| Path::new(url).extension().is_none())
            .filter_map(|s| Self::normalize_url(s, given_url))
            .collect()
    }

    fn normalize_url(url: &str, given_url: &str) -> Option<String> {
        let given_url = Url::parse(given_url).unwrap();
        Url::parse(url)
            .map(|new_url| {
                if new_url.has_host() && new_url.host_str() == given_url.host_str() {
                    Some(url.to_string())
                } else {
                    None
                }
            })
            .unwrap_or_else(|_| {
                if url.starts_with('/') {
                    Some(format!("{given_url}{url}"))
                } else {
                    None
                }
            })
    }

    fn save_urls(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = fs::File::create("found_urls.txt")?;
        let mut text = String::new();
        for url in self.found_urls.iter() {
            text.push_str(url);
            text.push('\n');

            file.write( text.as_bytes() )?;

            text.clear();
        }
        println!("Saved Urls Found in `found_urls.txt`");
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    if args.len() != 2 {
        eprintln!("Usage: crawler <URL>");
        return Ok(());
    }
    args.next();
    let origin_url = args.next().unwrap();

    if origin_url.starts_with("http") {
        let mut crawler = Crawler::new()?;
        crawler.crawl(origin_url.as_str())?;
    } else {
        eprintln!("Error: Not Valid URL");
    }

    Ok(())
}
