// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { APIContentCode } from "./APIContentCode";
import type { APIContentImage } from "./APIContentImage";
import type { APIContentLink } from "./APIContentLink";

export type APIFileContent = { "Heading": string } | { "Paragraph": string } | { "Image": APIContentImage } | { "Link": APIContentLink } | { "BulletList": Array<string> } | { "Code": APIContentCode };