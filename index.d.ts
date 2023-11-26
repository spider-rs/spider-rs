/* tslint:disable */
/* eslint-disable */

/* auto-generated by NAPI-RS */

/** a simple page object dummy */
export interface Page {
  /** the url found */
  url: string
  /** the content of the page found */
  content: string
}
/** crawl a website gathering all links to array */
export function collectAllLinks(n: string): Promise<Website>
export class Website {
  /** all of the website links. */
  links: Array<string>
  /** the pages found */
  pages: Array<Page>
}
