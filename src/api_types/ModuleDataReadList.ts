// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { AIIntegration } from "./AIIntegration";
import type { Chat } from "./Chat";
import type { DatabaseSource } from "./DatabaseSource";
import type { DirectorySource } from "./DirectorySource";
import type { EmailAccount } from "./EmailAccount";
import type { OAuth2 } from "./OAuth2";
import type { UserAccount } from "./UserAccount";

export type ModuleDataReadList =
  | { type: "UserAccount"; data: Array<UserAccount> }
  | { type: "DirectorySource"; data: Array<DirectorySource> }
  | { type: "DatabaseSource"; data: Array<DatabaseSource> }
  | { type: "AIIntegration"; data: Array<AIIntegration> }
  | { type: "Chat"; data: Array<Chat> }
  | { type: "OAuth2"; data: Array<OAuth2> }
  | { type: "EmailAccount"; data: Array<EmailAccount> };
