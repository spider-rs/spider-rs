use crate::conversions::{object_to_u8, ObjectConvert};
use crate::{NPage, BUFFER};
use indexmap::IndexMap;
use napi::{bindgen_prelude::Object, tokio::task::JoinHandle};
use napi::{Env, JsUnknown};
use spider::compact_str::CompactString;
use spider::configuration::{WaitForDelay, WaitForIdleNetwork, WaitForSelector};
use spider::{configuration::RedirectPolicy, utils::shutdown};
use std::time::Duration;

#[napi]
/// a website holding the inner spider::website::Website from Rust fit for nodejs.
pub struct Website {
  /// the website from spider.
  inner: spider::website::Website,
  /// spawned subscription handles.
  subscription_handles: IndexMap<u32, JoinHandle<()>>,
  /// spawned crawl handles.
  crawl_handles: IndexMap<u32, JoinHandle<()>>,
  /// do not convert content to UT8.
  raw_content: bool,
  /// the data collected.
  collected_data: Box<Vec<Vec<u8>>>,
  /// is the crawl running in the background.
  running_in_background: bool, // /// the file handle for storing data
                               // file_handle: Option<spider::tokio::fs::File>,
}

#[napi(object)]
struct PageEvent {
  pub page: NPage,
}

#[napi]
impl Website {
  #[napi(constructor)]
  /// a new website.
  pub fn new(url: String, raw_content: Option<bool>) -> Self {
    Website {
      inner: spider::website::Website::new(&url),
      subscription_handles: IndexMap::new(),
      crawl_handles: IndexMap::new(),
      raw_content: raw_content.unwrap_or_default(),
      collected_data: Box::new(Vec::new()),
      running_in_background: false, // file_handle: None,
    }
  }

  /// Get the crawl status.
  #[napi(getter)]
  pub fn status(&self) -> String {
    use std::string::ToString;
    self.inner.get_status().to_string()
  }

  #[napi]
  /// Store data to heap memory. The data must be an object. Use `website.export_jsonl_data` to store to disk. When using this method test occordingly since only certain primitives are supported.
  pub fn push_data(&mut self, env: Env, obj: JsUnknown) -> napi::Result<()> {
    match env.from_js_value::<serde_json::Value, &JsUnknown>(&obj) {
      Ok(deserialized) => {
        self
          .collected_data
          .push(object_to_u8(ObjectConvert::Val(deserialized))?);
      }
      _ => match obj.coerce_to_object() {
        Ok(obj) => {
          self
            .collected_data
            .push(object_to_u8(ObjectConvert::Obj(obj))?);
        }
        _ => (),
      },
    }

    Ok(())
  }

  #[napi]
  /// Clear the collected data from heap memory. This only handles the data from `website.pushData`.
  pub fn clear_data(&mut self) -> napi::Result<()> {
    self.collected_data.clear();
    Ok(())
  }

  #[napi]
  /// read the data from the heap memory.
  pub fn read_data(&mut self) -> serde_json::Value {
    self
      .collected_data
      .iter()
      .map(|d| serde_json::from_slice::<serde_json::Value>(d).unwrap_or_default())
      .collect()
  }

  #[napi]
  /// store data to memory for disk storing. This will create the path if not exist and defaults to ./storage.
  pub async fn export_jsonl_data(&self, export_path: Option<String>) -> napi::Result<()> {
    use napi::tokio::io::AsyncWriteExt;
    let file = match export_path {
      Some(p) => {
        let base_dir = p
          .split("/")
          .into_iter()
          .map(|f| {
            if f.contains(".") {
              "".to_string()
            } else {
              f.to_string()
            }
          })
          .collect::<String>();

        spider::tokio::fs::create_dir_all(&base_dir).await?;

        if !p.contains(".") {
          p + ".jsonl"
        } else {
          p
        }
      }
      _ => {
        spider::tokio::fs::create_dir_all("./storage").await?;
        "./storage/".to_owned()
          + &self
            .inner
            .get_url()
            .inner()
            .replace("http://", "")
            .replace("https://", "")
          + "jsonl"
      }
    };
    let mut file = spider::tokio::fs::File::create(file).await?;

    for (index, data) in self.collected_data.iter().enumerate() {
      if index > 0 {
        file.write_all(b"\n").await?;
      }
      // transform data step needed to auto convert type ..
      file.write_all(&data).await?;
    }

    Ok(())
  }

  #[napi]
  /// subscribe and add an event listener.
  pub fn subscribe(
    &mut self,
    on_page_event: napi::threadsafe_function::ThreadsafeFunction<NPage>,
  ) -> u32 {
    let mut rx2 = self
      .inner
      .subscribe(*BUFFER / 2)
      .expect("sync feature should be enabled");
    let raw_content = self.raw_content;

    let handle = spider::tokio::spawn(async move {
      while let Ok(res) = rx2.recv().await {
        on_page_event.call(
          Ok(NPage::new(&res, raw_content)),
          napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
        );
      }
    });

    // always return the highest value as the next id.
    let id = match self.subscription_handles.last() {
      Some(handle) => handle.0 + 1,
      _ => 0,
    };

    self.subscription_handles.insert(id, handle);

    id
  }

  #[napi]
  /// remove a subscription listener.
  pub fn unsubscribe(&mut self, id: Option<u32>) -> bool {
    match id {
      Some(id) => {
        let handle = self.subscription_handles.get(&id);

        match handle {
          Some(h) => {
            h.abort();
            self.subscription_handles.shift_remove_entry(&id);
            true
          }
          _ => false,
        }
      }
      // we may want to get all subs and remove them
      _ => {
        let keys = self.subscription_handles.len();
        for k in self.subscription_handles.drain(..) {
          k.1.abort();
        }
        keys > 0
      }
    }
  }

  #[napi]
  /// stop a crawl
  pub async unsafe fn stop(&mut self, id: Option<u32>) -> bool {
    self.inner.stop();

    // prevent the last background run
    if self.running_in_background {
      // we may want ID's to be used as an option along with urls for complete shutdowns.
      shutdown(self.inner.get_url().inner()).await;
      self.running_in_background = false;
    }

    match id {
      Some(id) => {
        let handle = self.crawl_handles.get(&id);

        match handle {
          Some(h) => {
            h.abort();
            self.crawl_handles.shift_remove_entry(&id);
            true
          }
          _ => false,
        }
      }
      _ => {
        let keys = self.crawl_handles.len();
        for k in self.crawl_handles.drain(..) {
          k.1.abort();
        }
        keys > 0
      }
    }
  }

  #[napi]
  /// crawl a website
  pub async unsafe fn crawl(
    &mut self,
    on_page_event: Option<napi::threadsafe_function::ThreadsafeFunction<NPage>>,
    background: Option<bool>,
    headless: Option<bool>,
  ) {
    // only run in background if on_page_event is handled for streaming.
    let background = background.is_some() && background.unwrap_or_default();
    let headless = headless.is_some() && headless.unwrap_or_default();
    let raw_content = self.raw_content;

    if background {
      self.running_in_background = background;
    }

    match on_page_event {
      Some(callback) => {
        if background {
          let mut website = self.inner.clone();
          let mut rx2 = website
            .subscribe(*BUFFER / 2)
            .expect("sync feature should be enabled");

          let handle = spider::tokio::spawn(async move {
            while let Ok(res) = rx2.recv().await {
              callback.call(
                Ok(NPage::new(&res, raw_content)),
                napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
              );
            }
          });

          let crawl_id = match self.crawl_handles.last() {
            Some(handle) => handle.0 + 1,
            _ => 0,
          };

          let crawl_handle = spider::tokio::spawn(async move {
            if headless {
              website.crawl().await;
            } else {
              website.crawl_raw().await;
            }
          });

          let id = match self.subscription_handles.last() {
            Some(handle) => handle.0 + 1,
            _ => 0,
          };

          self.crawl_handles.insert(crawl_id, crawl_handle);
          self.subscription_handles.insert(id, handle);
        } else {
          let mut rx2 = self
            .inner
            .subscribe(*BUFFER / 2)
            .expect("sync feature should be enabled");

          let handle = spider::tokio::spawn(async move {
            while let Ok(res) = rx2.recv().await {
              callback.call(
                Ok(NPage::new(&res, raw_content)),
                napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
              );
            }
          });

          if headless {
            self.inner.crawl().await;
          } else {
            self.inner.crawl_raw().await;
          }

          let id = match self.subscription_handles.last() {
            Some(handle) => handle.0 + 1,
            _ => 0,
          };

          self.subscription_handles.insert(id, handle);
        }
      }
      _ => {
        if background {
          let mut website = self.inner.clone();

          let crawl_id = match self.crawl_handles.last() {
            Some(handle) => handle.0 + 1,
            _ => 0,
          };

          let crawl_handle = spider::tokio::spawn(async move {
            if headless {
              website.crawl().await;
            } else {
              website.crawl_raw().await;
            }
          });

          self.crawl_handles.insert(crawl_id, crawl_handle);
        } else {
          if headless {
            self.inner.crawl().await;
          } else {
            self.inner.crawl_raw().await;
          }
        }
      }
    }
  }

  #[napi]
  /// Start to crawl website with async concurrency smart. Use HTTP first and JavaScript Rendering as needed.
  pub async unsafe fn crawl_smart(
    &mut self,
    on_page_event: Option<napi::threadsafe_function::ThreadsafeFunction<NPage>>,
    background: Option<bool>,
  ) {
    // only run in background if on_page_event is handled for streaming.
    let background = background.is_some() && background.unwrap_or_default();
    let raw_content = self.raw_content;

    if background {
      self.running_in_background = background;
    }

    match on_page_event {
      Some(callback) => {
        if background {
          let mut website = self.inner.clone();
          let mut rx2 = website
            .subscribe(*BUFFER / 2)
            .expect("sync feature should be enabled");

          let handle = spider::tokio::spawn(async move {
            while let Ok(res) = rx2.recv().await {
              callback.call(
                Ok(NPage::new(&res, raw_content)),
                napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
              );
            }
          });

          let crawl_id = match self.crawl_handles.last() {
            Some(handle) => handle.0 + 1,
            _ => 0,
          };

          let crawl_handle = spider::tokio::spawn(async move {
            website.crawl_smart().await;
          });

          let id = match self.subscription_handles.last() {
            Some(handle) => handle.0 + 1,
            _ => 0,
          };

          self.crawl_handles.insert(crawl_id, crawl_handle);
          self.subscription_handles.insert(id, handle);
        } else {
          let mut rx2 = self
            .inner
            .subscribe(*BUFFER / 2)
            .expect("sync feature should be enabled");

          let handle = spider::tokio::spawn(async move {
            while let Ok(res) = rx2.recv().await {
              callback.call(
                Ok(NPage::new(&res, raw_content)),
                napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
              );
            }
          });

          self.inner.crawl_smart().await;
          let _ = handle.await;
        }
      }
      _ => {
        if background {
          let mut website = self.inner.clone();

          let crawl_id = match self.crawl_handles.last() {
            Some(handle) => handle.0 + 1,
            _ => 0,
          };

          let crawl_handle = spider::tokio::spawn(async move {
            website.crawl_smart().await;
          });

          self.crawl_handles.insert(crawl_id, crawl_handle);
        } else {
          self.inner.crawl_smart().await;
        }
      }
    }
  }

  #[napi]
  /// scrape a website
  pub async unsafe fn scrape(
    &mut self,
    on_page_event: Option<napi::threadsafe_function::ThreadsafeFunction<NPage>>,
    background: Option<bool>,
    headless: Option<bool>,
  ) {
    let headless = headless.is_some() && headless.unwrap_or_default();
    let raw_content = self.raw_content;
    let background = background.is_some() && background.unwrap_or_default();

    if background {
      self.running_in_background = background;
    }

    match on_page_event {
      Some(callback) => {
        if background {
          let mut website = self.inner.clone();
          let mut rx2 = website
            .subscribe(*BUFFER / 2)
            .expect("sync feature should be enabled");

          let handle = spider::tokio::spawn(async move {
            while let Ok(res) = rx2.recv().await {
              callback.call(
                Ok(NPage::new(&res, raw_content)),
                napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
              );
            }
          });

          let crawl_id = match self.crawl_handles.last() {
            Some(handle) => handle.0 + 1,
            _ => 0,
          };

          let crawl_handle = spider::tokio::spawn(async move {
            if headless {
              website.scrape().await;
            } else {
              website.scrape_raw().await;
            }
          });

          let id = match self.subscription_handles.last() {
            Some(handle) => handle.0 + 1,
            _ => 0,
          };

          self.crawl_handles.insert(crawl_id, crawl_handle);
          self.subscription_handles.insert(id, handle);
        } else {
          let mut rx2 = self
            .inner
            .subscribe(*BUFFER / 2)
            .expect("sync feature should be enabled");

          let handle = spider::tokio::spawn(async move {
            while let Ok(res) = rx2.recv().await {
              callback.call(
                Ok(NPage::new(&res, raw_content)),
                napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
              );
            }
          });

          if headless {
            self.inner.scrape().await;
          } else {
            self.inner.scrape_raw().await;
          }

          let _ = handle.await;
        }
      }
      _ => {
        if background {
          let mut website = self.inner.clone();

          let crawl_id = match self.crawl_handles.last() {
            Some(handle) => handle.0 + 1,
            _ => 0,
          };

          let crawl_handle = spider::tokio::spawn(async move {
            if headless {
              website.scrape().await;
            } else {
              website.scrape_raw().await;
            }
          });

          self.crawl_handles.insert(crawl_id, crawl_handle);
        } else {
          if headless {
            self.inner.scrape().await;
          } else {
            self.inner.scrape_raw().await;
          }
        }
      }
    }
  }

  /// run a cron job
  #[napi]
  pub async unsafe fn run_cron(
    &mut self,
    on_page_event: Option<napi::threadsafe_function::ThreadsafeFunction<NPage>>,
  ) -> Cron {
    let cron_handle = match on_page_event {
      Some(callback) => {
        let mut rx2 = self
          .inner
          .subscribe(*BUFFER / 2)
          .expect("sync feature should be enabled");
        let raw_content = self.raw_content;

        let handler = spider::tokio::spawn(async move {
          while let Ok(res) = rx2.recv().await {
            callback.call(
              Ok(NPage::new(&res, raw_content)),
              napi::threadsafe_function::ThreadsafeFunctionCallMode::NonBlocking,
            );
          }
        });

        Some(handler)
      }
      _ => None,
    };

    let inner = self.inner.run_cron().await;

    Cron { inner, cron_handle }
  }

  #[napi]
  /// get all the links of a website
  pub fn get_links(&self) -> Vec<String> {
    let links = self
      .inner
      .get_links()
      .iter()
      .map(|x| x.as_ref().to_string())
      .collect::<Vec<String>>();
    links
  }

  #[napi(getter)]
  /// get the size of the website in amount of pages crawled. If you ran the page in the background, this value will not update.
  pub fn size(&mut self) -> u32 {
    self.inner.size() as u32
  }

  /// get all the pages of a website - requires calling website.scrape
  #[napi]
  pub fn get_pages(&self) -> Vec<NPage> {
    let mut pages: Vec<NPage> = Vec::new();
    let raw_content = self.raw_content;

    match self.inner.get_pages() {
      Some(p) => {
        for page in p.iter() {
          pages.push(NPage::new(page, raw_content));
        }
      }
      _ => (),
    }

    pages
  }

  #[napi]
  /// drain all links from storing
  pub fn drain_links(&mut self) -> Vec<String> {
    let links = self
      .inner
      .get_links()
      .iter()
      .map(|x| x.as_ref().to_string())
      .collect::<Vec<String>>();
    self.inner.drain_links();
    links
  }

  #[napi]
  /// clear all links and page data
  pub fn clear(&mut self) {
    self.inner.clear();
  }

  #[napi]
  /// Set HTTP headers for request using [reqwest::header::HeaderMap](https://docs.rs/reqwest/latest/reqwest/header/struct.HeaderMap.html).
  pub fn with_headers(&mut self, headers: Option<Object>) -> &Self {
    use std::str::FromStr;

    match headers {
      Some(obj) => {
        let mut h = spider::reqwest::header::HeaderMap::new();
        let keys = Object::keys(&obj).unwrap_or_default();

        for key in keys.into_iter() {
          let header_key = spider::reqwest::header::HeaderName::from_str(&key);

          match header_key {
            Ok(hn) => {
              let header_value = obj
                .get::<String, String>(key)
                .unwrap_or_default()
                .unwrap_or_default();

              match spider::reqwest::header::HeaderValue::from_str(&header_value) {
                Ok(hk) => {
                  h.append(hn, hk);
                }
                _ => (),
              }
            }
            _ => (),
          }
        }
        self.inner.with_headers(Some(h));
      }
      _ => {
        self.inner.with_headers(None);
      }
    };

    self
  }

  /// Add user agent to request.
  #[napi]
  pub fn with_user_agent(&mut self, user_agent: Option<&str>) -> &Self {
    self.inner.configuration.with_user_agent(user_agent);
    self
  }

  /// Respect robots.txt file.
  #[napi]
  pub fn with_respect_robots_txt(&mut self, respect_robots_txt: bool) -> &Self {
    self
      .inner
      .configuration
      .with_respect_robots_txt(respect_robots_txt);
    self
  }

  /// Determine whether to collect all the resources found on pages.
  #[napi]
  pub fn with_full_resources(&mut self, full_resources: bool) -> &Self {
    self.inner.configuration.with_full_resources(full_resources);
    self
  }

  /// Use network interception for the request to only allow content that matches the host. If the content is from a 3rd party it needs to be part of our include list.
  #[napi]
  pub fn with_chrome_intercept(&mut self, chrome_intercept: bool, block_images: bool) -> &Self {
    let mut intercept_config =
      spider::features::chrome_common::RequestInterceptConfiguration::new(chrome_intercept);

    intercept_config.block_visuals = block_images;

    self.inner.with_chrome_intercept(intercept_config);
    self
  }

  /// Set the connection url for the chrome instance. This method does nothing if the `chrome` is not enabled.
  #[napi]
  pub fn with_chrome_connection(&mut self, chrome_connection: String) -> &Self {
    self
      .inner
      .with_chrome_connection(if chrome_connection.is_empty() {
        None
      } else {
        Some(chrome_connection)
      });
    self
  }

  /// Preserve the HOST header.
  #[napi]
  pub fn with_preserve_host_header(&mut self, preserve_host: bool) -> &Self {
    self.inner.with_preserve_host_header(preserve_host);
    self
  }

  /// Include subdomains detection.
  #[napi]
  pub fn with_subdomains(&mut self, subdomains: bool) -> &Self {
    self.inner.configuration.with_subdomains(subdomains);
    self
  }

  /// Include tld detection.
  #[napi]
  pub fn with_tld(&mut self, tld: bool) -> &Self {
    self.inner.configuration.with_tld(tld);
    self
  }

  /// Only use HTTP/2.
  #[napi]
  pub fn with_http2_prior_knowledge(&mut self, http2_prior_knowledge: bool) -> &Self {
    self
      .inner
      .configuration
      .with_http2_prior_knowledge(http2_prior_knowledge);
    self
  }

  /// Max time to wait for request duration to milliseconds.
  #[napi]
  pub fn with_request_timeout(&mut self, request_timeout: Option<u32>) -> &Self {
    self
      .inner
      .configuration
      .with_request_timeout(match request_timeout {
        Some(d) => Some(Duration::from_millis(d.into())),
        _ => None,
      });
    self
  }

  /// add external domains
  #[napi]
  pub fn with_external_domains(&mut self, external_domains: Option<Vec<String>>) -> &Self {
    self.inner.with_external_domains(match external_domains {
      Some(ext) => Some(ext.into_iter()),
      _ => None,
    });
    self
  }

  /// Use stealth mode for the request. This does nothing without chrome.
  #[napi]
  pub fn with_stealth(&mut self, stealth_mode: Option<bool>) -> &Self {
    self.inner.with_stealth(match stealth_mode {
      Some(ext) => ext,
      _ => false,
    });
    self
  }

  /// Dangerously accept invalid certificates - this should be used as a last resort.
  #[napi]
  pub fn with_danger_accept_invalid_certs(&mut self, accept_invalid_certs: Option<bool>) -> &Self {
    self
      .inner
      .with_danger_accept_invalid_certs(match accept_invalid_certs {
        Some(ext) => ext,
        _ => false,
      });
    self
  }

  #[napi]
  /// Set the crawling budget
  pub fn with_budget(&mut self, budget: Option<std::collections::HashMap<String, u32>>) -> &Self {
    use spider::hashbrown::hash_map::HashMap;

    match budget {
      Some(d) => {
        self.inner.with_budget(Some(
          d.iter()
            .map(|(k, v)| (k.as_str(), *v))
            .collect::<HashMap<&str, u32>>(),
        ));
      }
      _ => (),
    }

    self
  }

  /// Set the max redirects allowed for request.
  #[napi]
  pub fn with_redirect_limit(&mut self, redirect_limit: u32) -> &Self {
    self.inner.with_redirect_limit(redirect_limit as usize);
    self
  }

  /// Set the redirect policy to use, either Strict or Loose by default.
  #[napi]
  pub fn with_redirect_policy(&mut self, strict: bool) -> &Self {
    self.inner.with_redirect_policy(if strict {
      RedirectPolicy::Strict
    } else {
      RedirectPolicy::Loose
    });
    self
  }

  #[napi]
  /// Regex blacklist urls from the crawl
  pub fn with_blacklist_url(&mut self, blacklist_url: Option<Vec<String>>) -> &Self {
    self
      .inner
      .configuration
      .with_blacklist_url(match blacklist_url {
        Some(v) => {
          let mut blacklist: Vec<CompactString> = Vec::new();
          for item in v {
            blacklist.push(CompactString::new(item));
          }
          Some(blacklist)
        }
        _ => None,
      });

    self
  }

  #[napi]
  /// Regex whitelist urls from the crawl
  pub fn with_whitelist_url(&mut self, whitelist_url: Option<Vec<String>>) -> &Self {
    self
      .inner
      .configuration
      .with_whitelist_url(match whitelist_url {
        Some(v) => {
          let mut whitelist: Vec<CompactString> = Vec::new();
          for item in v {
            whitelist.push(CompactString::new(item));
          }
          Some(whitelist)
        }
        _ => None,
      });

    self
  }

  #[napi]
  /// Wait for a delay. Should only be used for testing. This method does nothing if the `chrome` feature is not enabled.
  pub fn with_wait_for_delay(&mut self, seconds: Option<u32>, nanos: Option<u32>) -> &Self {
    self
      .inner
      .configuration
      .with_wait_for_delay(if seconds.is_some() || nanos.is_some() {
        let duration = Duration::new(
          seconds.unwrap_or_default() as u64,
          nanos.unwrap_or_default(),
        );
        Some(WaitForDelay::new(Some(duration)))
      } else {
        None
      });

    self
  }

  #[napi]
  /// Wait for a CSS query selector. This method does nothing if the `chrome` feature is not enabled.
  pub fn with_wait_for_selector(
    &mut self,
    selector: Option<&str>,
    seconds: Option<u32>,
    nanos: Option<u32>,
  ) -> &Self {
    self
      .inner
      .configuration
      .with_wait_for_selector(if seconds.is_some() || nanos.is_some() {
        let duration = Duration::new(
          seconds.unwrap_or_default() as u64,
          nanos.unwrap_or_default(),
        );
        Some(WaitForSelector::new(
          Some(duration),
          selector.unwrap_or_default().to_string(),
        ))
      } else {
        None
      });

    self
  }

  #[napi]
  /// Wait for idle network request. This method does nothing if the `chrome` feature is not enabled.
  pub fn with_wait_for_idle_network(&mut self, seconds: Option<u32>, nanos: Option<u32>) -> &Self {
    self
      .inner
      .configuration
      .with_wait_for_idle_network(if seconds.is_some() || nanos.is_some() {
        let duration = Duration::new(
          seconds.unwrap_or_default() as u64,
          nanos.unwrap_or_default(),
        );
        Some(WaitForIdleNetwork::new(Some(duration)))
      } else {
        None
      });

    self
  }

  /// Setup cron jobs to run
  #[napi]
  pub fn with_cron(&mut self, cron_str: String, cron_type: Option<String>) -> &Self {
    self.inner.with_cron(
      cron_str.as_str(),
      if cron_type.unwrap_or_default() == "scrape" {
        spider::website::CronType::Scrape
      } else {
        spider::website::CronType::Crawl
      },
    );
    self
  }

  /// Use OpenAI to generate dynamic javascript snippets. Make sure to set the `OPENAI_API_KEY` env variable.
  #[napi]
  pub fn with_openai(&mut self, env: Env, openai_configs: Option<napi::JsObject>) -> &Self {
    use serde_json::Value;
    use spider::configuration::GPTConfigs;
    let openai_configs: Option<Value> = match openai_configs {
      Some(obj) => match env.from_js_value(obj) {
        Ok(e) => Some(e),
        _ => None,
      },
      None => None,
    };

    if let Some(configs) = openai_configs {
      let configs: GPTConfigs =
        serde_json::from_value(configs).unwrap_or_else(|_| GPTConfigs::default());

      if !configs.model.is_empty() || configs.prompt_url_map.is_some() {
        self.inner.with_openai(Some(configs));
      }
    }

    self
  }

  /// Take screenshots of web pages using chrome.
  #[napi]
  pub fn with_screenshot(
    &mut self,
    env: Env,

    #[napi(ts_arg_type = r#"{
  /** The screenshot params. */
  params: {
    /** Chrome DevTools Protocol screenshot options. */
    cdp_params: {
      /** Image compression format (defaults to png). */
      format: 'jpeg' | 'png' | 'webp'
      /** Compression quality from range [0..100] (jpeg only). */
      quality: number
      /** Capture the screenshot of a given region only. */
      clip: {
        x: number
        y: number
        height: number
        width: number
        scale: number
      }
      /** Capture the screenshot from the surface, rather than the view. Defaults to true.*/
      from_surface: boolean
      /** Capture the screenshot beyond the viewport. Defaults to false. */
      capture_beyond_viewport: boolean
    }
    /** Take full page screenshot */
    full_page: boolean
    /** Make the background transparent (png only). */
    omit_background: boolean
  }
  /** Return the bytes of the screenshot on the Page. */
  bytes: boolean
  /** Store the screenshot to disk. This can be used with output_dir. If disabled will not store the file to the output directory. */
  save: boolean
  /** The output directory to store the file. Parent folders may be created inside the directory. */
  output_dir: string | null
}"#)]
    screenshot_configs: Option<napi::JsObject>,
  ) -> &Self {
    use serde_json::Value;
    use spider::configuration::ScreenShotConfig;
    let screenshot_configs: Option<Value> = match screenshot_configs {
      Some(obj) => match env.from_js_value(obj) {
        Ok(e) => Some(e),
        _ => None,
      },
      None => None,
    };

    if let Some(configs) = screenshot_configs {
      let configs: ScreenShotConfig =
        serde_json::from_value(configs).unwrap_or_else(|_| ScreenShotConfig::default());

      self.inner.with_screenshot(Some(configs));
    }

    self
  }

  /// Delay between request as ms.
  #[napi]
  pub fn with_delay(&mut self, delay: u32) -> &Self {
    self.inner.configuration.with_delay(delay.into());
    self
  }

  /// Set a crawl depth limit. If the value is 0 there is no limit.
  #[napi]
  pub fn with_depth(&mut self, depth: u32) -> &Self {
    self.inner.configuration.with_depth(depth as usize);
    self
  }

  /// Return the links found on the page in the channel subscriptions. This method does nothing if the `decentralized` is enabled.
  #[napi]
  pub fn with_return_page_links(&mut self, return_page_links: bool) -> &Self {
    self
      .inner
      .configuration
      .with_return_page_links(return_page_links);
    self
  }

  /// Cache the page following HTTP rules.
  #[napi]
  pub fn with_caching(&mut self, cache: bool) -> &Self {
    self.inner.configuration.with_caching(cache);
    self
  }

  /// Set the sitemap url.
  #[napi]
  pub fn with_sitemap(&mut self, sitemap: Option<&str>) -> &Self {
    self.inner.configuration.with_sitemap(sitemap);
    self
  }

  /// Use proxies for request.
  #[napi]
  pub fn with_proxies(&mut self, proxies: Option<Vec<String>>) -> &Self {
    self.inner.configuration.with_proxies(proxies);
    self
  }

  #[napi]
  /// build the inner website - not required for all builder_steps
  pub fn build(&mut self) -> &Self {
    match self.inner.build() {
      Ok(w) => self.inner = w,
      _ => (),
    }
    self
  }
}

/// a runner for handling crons
#[napi]
pub struct Cron {
  /// the runner task
  inner: spider::async_job::Runner,
  /// inner cron handle
  cron_handle: Option<JoinHandle<()>>,
}

#[napi]
impl Cron {
  /// stop the cron instance
  #[napi]
  pub async unsafe fn stop(&mut self) {
    self.inner.stop().await;
    match &self.cron_handle {
      Some(h) => h.abort(),
      _ => (),
    }
  }
}
