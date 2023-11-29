/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

/** a simple page object */
export interface NPage {
  /** the url found */
  url: string
  /** the content of the page found */
  content: string
}
/** crawl a website using HTTP gathering all links and html. */
export function crawl(url: string): Promise<NWebsite>
/** a simple page object */
export class Page {
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
/** website main data from rust to node */
export class NWebsite {
  /** all of the website links. */
  links: Array<string>
  /** the pages found */
  pages: Array<NPage>
}
/** a website holding the inner spider::website::Website from Rust fit for nodejs */
export class Website {
  /** a new website */
  constructor(url: string)
  /** Get the crawl status */
  getStatus(): string
  /** subscribe and add an event listener */
  subscribe(onPageEvent: (err: Error | null, value: NPage) => any): number
  /** remove a subscription listener */
  unsubscribe(id?: number | undefined | null): boolean
  /** crawl a website */
  crawl(onPageEvent?: (err: Error | null, value: NPage) => any | undefined | null, background?: boolean | undefined | null, headless?: boolean | undefined | null): Promise<void>
  /** scrape a website */
  scrape(onPageEvent?: (err: Error | null, value: NPage) => any | undefined | null, background?: boolean | undefined | null, headless?: boolean | undefined | null): Promise<void>
  /** run the cron */
  runCron(onPageEvent?: (err: Error | null, value: NPage) => any | undefined | null): Promise<Cron>
  /** get all the links of a website */
  getLinks(): Array<string>
  /** get all the pages of a website - requires calling website.scrape */
  getPages(): Array<NPage>
  /** Set HTTP headers for request using [reqwest::header::HeaderMap](https://docs.rs/reqwest/latest/reqwest/header/struct.HeaderMap.html). */
  withHeaders(headers?: object | undefined | null): this
  /** Add user agent to request. */
  withUserAgent(userAgent?: string | undefined | null): this
  /** Respect robots.txt file. */
  withRespectRobotsTxt(respectRobotsTxt: boolean): this
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
  /** Set the crawling budget */
  withBudget(budget?: Record<string, number> | undefined | null): this
  /** Regex black list urls from the crawl */
  withBlacklistUrl(blacklistUrl?: Array<string> | undefined | null): this
  /** Setup cron jobs to run */
  withCron(cronStr: string, cronType?: string | undefined | null): this
  /** Delay between request as ms. */
  withDelay(delay: number): this
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
