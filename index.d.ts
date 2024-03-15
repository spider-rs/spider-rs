/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

/** a simple page object */
export interface NPage {
  /** the url found. */
  url: string
  /** the content of the page found. */
  content: string
  /** the HTTP status code. */
  statusCode: number
  /** the raw content */
  rawContent?: Buffer
}
/** get the page title. */
export function pageTitle(page: NPage): string
/** crawl a website using HTTP gathering all links and html. */
export function crawl(url: string, rawContent?: boolean | undefined | null): Promise<NWebsite>
export interface PageEvent {
  page: NPage
}
/** website main data from rust to node. */
export class NWebsite {
  /** all of the website links. */
  links: Array<string>
  /** the pages found. */
  pages: Array<NPage>
}
/** a simple page object */
export class Page {
  /** the url for the page */
  url: string
  subdomains?: boolean
  tld?: boolean
  statusCode: number
  /** a new page */
  constructor(url: string, subdomains?: boolean | undefined | null, tld?: boolean | undefined | null)
  /** get the page content */
  fetch(): Promise<this>
  /** all links on the page */
  getLinks(): Promise<Array<string>>
  /** get the html for the page */
  getHtml(): string
  /** get the bytes for the page */
  getBytes(): any
}
/** a website holding the inner spider::website::Website from Rust fit for nodejs. */
export class Website {
  /** a new website. */
  constructor(url: string, rawContent?: boolean | undefined | null)
  /** Get the crawl status. */
  get status(): string
  /** Store data to heap memory. The data must be an object. Use `website.export_jsonl_data` to store to disk. When using this method test occordingly since only certain primitives are supported. */
  pushData(obj: unknown): void
  /** Clear the collected data from heap memory. This only handles the data from `website.pushData`. */
  clearData(): void
  /** read the data from the heap memory. */
  readData(): any
  /** store data to memory for disk storing. This will create the path if not exist and defaults to ./storage. */
  exportJsonlData(exportPath?: string | undefined | null): Promise<void>
  /** subscribe and add an event listener. */
  subscribe(onPageEvent: (err: Error | null, arg: NPage) => any): number
  /** remove a subscription listener. */
  unsubscribe(id?: number | undefined | null): boolean
  /** stop a crawl */
  stop(id?: number | undefined | null): Promise<boolean>
  /** crawl a website */
  crawl(onPageEvent?: (err: Error | null, arg: NPage) => any | undefined | null, background?: boolean | undefined | null, headless?: boolean | undefined | null): Promise<void>
  /** Start to crawl website with async concurrency smart. Use HTTP first and JavaScript Rendering as needed. */
  crawlSmart(onPageEvent?: (err: Error | null, arg: NPage) => any | undefined | null, background?: boolean | undefined | null): Promise<void>
  /** scrape a website */
  scrape(onPageEvent?: (err: Error | null, arg: NPage) => any | undefined | null, background?: boolean | undefined | null, headless?: boolean | undefined | null): Promise<void>
  /** run a cron job */
  runCron(onPageEvent?: (err: Error | null, arg: NPage) => any | undefined | null): Promise<Cron>
  /** get all the links of a website */
  getLinks(): Array<string>
  /** get the size of the website in amount of pages crawled. If you ran the page in the background, this value will not update. */
  get size(): number
  /** get all the pages of a website - requires calling website.scrape */
  getPages(): Array<NPage>
  /** drain all links from storing */
  drainLinks(): Array<string>
  /** clear all links and page data */
  clear(): void
  /** Set HTTP headers for request using [reqwest::header::HeaderMap](https://docs.rs/reqwest/latest/reqwest/header/struct.HeaderMap.html). */
  withHeaders(headers?: object | undefined | null): this
  /** Add user agent to request. */
  withUserAgent(userAgent?: string | undefined | null): this
  /** Respect robots.txt file. */
  withRespectRobotsTxt(respectRobotsTxt: boolean): this
  /** Use network interception for the request to only allow content that matches the host. If the content is from a 3rd party it needs to be part of our include list. */
  withChromeIntercept(chromeIntercept: boolean, blockImages: boolean): this
  /** Include subdomains detection. */
  withSubdomains(subdomains: boolean): this
  /** Include tld detection. */
  withTld(tld: boolean): this
  /** Only use HTTP/2. */
  withHttp2PriorKnowledge(http2PriorKnowledge: boolean): this
  /** Max time to wait for request duration to milliseconds. */
  withRequestTimeout(requestTimeout?: number | undefined | null): this
  /** add external domains */
  withExternalDomains(externalDomains?: Array<string> | undefined | null): this
  /** Use stealth mode for the request. This does nothing without chrome. */
  withStealth(stealthMode?: boolean | undefined | null): this
  /** Set the crawling budget */
  withBudget(budget?: Record<string, number> | undefined | null): this
  /** Set the max redirects allowed for request. */
  withRedirectLimit(redirectLimit: number): this
  /** Set the redirect policy to use, either Strict or Loose by default. */
  withRedirectPolicy(strict: boolean): this
  /** Regex black list urls from the crawl */
  withBlacklistUrl(blacklistUrl?: Array<string> | undefined | null): this
  /** Setup cron jobs to run */
  withCron(cronStr: string, cronType?: string | undefined | null): this
  /** Delay between request as ms. */
  withDelay(delay: number): this
  /** Set a crawl depth limit. If the value is 0 there is no limit. */
  withDepth(depth: number): this
  /** Cache the page following HTTP rules. */
  withCaching(cache: boolean): this
  /** Set the sitemap url. */
  withSitemap(sitemap?: string | undefined | null): this
  /** Use proxies for request. */
  withProxies(proxies?: Array<string> | undefined | null): this
  /** build the inner website - not required for all builder_steps */
  build(): this
}
/** a runner for handling crons */
export class Cron {
  /** stop the cron instance */
  stop(): Promise<void>
}
