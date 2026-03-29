import type { ProjectStatus } from "./ProjectStatus";

/**
 * Project entity for managing work and hobby projects
 */
export type Project = { id: bigint, name: string, description: string, status: ProjectStatus, tasks_completed: number, tasks_total: number, 
/**
 * Date by which the project must be completed.
 * SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
 */
deadline_raw: string | null, deadline: bigint | null, notifications: number, created_at: bigint, updated_at: bigint, };


export type ProjectStatus = "active" | "planning" | "on-hold" | "completed" | "archived";


export type Event = { id: bigint, name: string, description: string | null, 
/**
 * Date of the event.
 * SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
 */
event_date_raw: string | null, event_date: bigint | null, location_id: bigint | null, 
/**
 * Person IDs of attendees (FK to persons table).
 */
attendees: Array<bigint>, project_id: bigint | null, task_id: bigint | null, 
/**
 * FK to the email this event was extracted from.
 */
source_email_id: bigint | null, created_at: bigint, updated_at: bigint, };


import type { Event } from "./Event";

export type EventsResponse = { events: Array<Event>, };


import type { TaskPriority } from "./TaskPriority";
import type { TaskStatus } from "./TaskStatus";

/**
 * Task entity for managing individual tasks
 */
export type Task = { id: bigint, project_id: bigint | null, title: string, description: string | null, status: TaskStatus, priority: TaskPriority, due_date: string | null, assigned_to: string | null, created_at: bigint, updated_at: bigint, };


export type TaskStatus = "todo" | "in-progress" | "review" | "done" | "cancelled";


export type TaskPriority = "low" | "medium" | "high" | "critical";


/**
 * Message in session response
 */
export type SessionMessage = { role: string, content: string, created_at: bigint, };


/**
 * Tool call in session response
 */
export type SessionToolCall = { tool_name: string, request: any, response: any, status: string, execution_time_ms: bigint | null, };


import type { SessionMessage } from "./SessionMessage";
import type { SessionToolCall } from "./SessionToolCall";

/**
 * Detailed session with messages and tool calls
 */
export type SessionResponse = { id: bigint, agent_name: string, provider: string, model: string, system_prompt: string | null, user_prompt: string, config: any, status: string, result: string | null, messages: Array<SessionMessage>, tool_calls: Array<SessionToolCall>, started_at: bigint, ended_at: bigint | null, };


/**
 * Simplified session info for list views
 */
export type SessionListItem = { id: bigint, agent_name: string, user_prompt: string, status: string, started_at: bigint, };


/**
 * Configuration for an AI provider API key
 */
export type AiProviderApiKeyConfig = { name: string, key: string | null, is_configured: boolean, };


/**
 * Configuration for an OAuth client app
 */
export type OAuthClientAppConfig = { provider: string, client_id: string | null, client_secret: string | null, is_configured: boolean, };


import type { AiProviderApiKeyConfig } from "./AiProviderApiKeyConfig";
import type { OAuthClientAppConfig } from "./OAuthClientAppConfig";

/**
 * Response for settings endpoint
 */
export type SettingsResponse = { config_file_path: string, ai_provider_api_keys: Array<AiProviderApiKeyConfig>, oauth_client_apps: Array<OAuthClientAppConfig>, projects_default_path: string | null, };


/**
 * Request to update AI provider API keys
 */
export type UpdateAiProviderApiKeysRequest = { openai_api_key: string | null, gemini_api_key: string | null, };


/**
 * Request to update OAuth client apps
 */
export type UpdateOAuthClientAppsRequest = { google_client_id: string | null, google_client_secret: string | null, };


export type CredentialType = "imap" | "smtp" | "oauth" | "apikey" | "database" | "localfile" | "custom";


import type { CredentialType } from "./CredentialType";

export type CreateCredentialRequest = { credential_type: CredentialType, identifier: string, username: string, 
/**
 * Password is optional for credential types that don't require keychain storage (e.g., LocalFile)
 */
password: string | null, service_name: string | null, port: number | null, use_tls: boolean | null, notes: string | null, extra_metadata: string | null, };


export type UpdateCredentialRequest = { username: string | null, password: string | null, service_name: string | null, port: number | null, use_tls: boolean | null, notes: string | null, extra_metadata: string | null, };


import type { CredentialType } from "./CredentialType";

export type CredentialMetadata = { id: bigint, credential_type: CredentialType, identifier: string, username: string, service_name: string | null, port: number | null, use_tls: boolean | null, notes: string | null, created_at: bigint, updated_at: bigint, last_accessed_at: bigint | null, is_active: boolean, extra_metadata: string | null, };


export type PasswordResponse = { password: string, };


import type { CredentialMetadata } from "./CredentialMetadata";

export type CredentialListResponse = { credentials: Array<CredentialMetadata>, };


export type ImapAuthMethod = "plain" | "oauth2" | "xoauth2";


import type { ImapAuthMethod } from "./ImapAuthMethod";

/**
 * IMAP-specific account settings
 */
export type ImapAccountSettings = { 
/**
 * IMAP server host (e.g., "imap.gmail.com")
 */
host: string, 
/**
 * IMAP server port (typically 993 for SSL, 143 for non-SSL)
 */
port: number, 
/**
 * Use TLS/SSL connection
 */
use_tls: boolean, 
/**
 * Authentication method
 */
auth_method: ImapAuthMethod, 
/**
 * Default mailbox/folder to monitor (default: "INBOX")
 */
default_mailbox: string, 
/**
 * Connection timeout in seconds
 */
connection_timeout_secs: number, 
/**
 * Whether to validate SSL certificates (should be true in production)
 */
validate_certs: boolean, };


import type { ImapAccountSettings } from "./ImapAccountSettings";

/**
 * Extended credential metadata with parsed IMAP settings
 */
export type ImapCredentialMetadata = { id: bigint, identifier: string, username: string, settings: ImapAccountSettings, notes: string | null, created_at: bigint, updated_at: bigint, last_accessed_at: bigint | null, is_active: boolean, };


/**
 * SMTP-specific account settings
 */
export type SmtpAccountSettings = { 
/**
 * SMTP server host (e.g., "smtp.gmail.com")
 */
host: string, 
/**
 * SMTP server port (typically 587 for STARTTLS, 465 for SSL)
 */
port: number, 
/**
 * Use TLS/SSL connection
 */
use_tls: boolean, 
/**
 * Connection timeout in seconds
 */
connection_timeout_secs: number, };


/**
 * API Key service settings
 */
export type ApiKeySettings = { 
/**
 * Base URL for the API (e.g., "https://api.stripe.com")
 */
base_url: string, 
/**
 * API version (if applicable)
 */
api_version: string | null, 
/**
 * Request timeout in seconds
 */
timeout_secs: number, };


/**
 * Local file path settings
 */
export type LocalFileSettings = { 
/**
 * Absolute path to the file or directory
 */
file_path: string, 
/**
 * Optional description of what this file contains
 */
description: string | null, 
/**
 * File type hint (e.g., "linkedin-archive", "email-export")
 */
file_type: string | null, };


export type EmailSyncDirection = "recent" | "backfill";


import type { EmailSyncDirection } from "./EmailSyncDirection";

/**
 * Request to trigger an email sync for a specific credential
 */
export type TriggerEmailSyncRequest = { credential_id: bigint, direction: EmailSyncDirection, };


import type { EmailAddress } from "./EmailAddress";

/**
 * Represents a stored email
 */
export type Email = { id: bigint, credential_id: bigint, uid: number, folder_id: bigint, message_id: string | null, subject: string | null, from_address: string, from_name: string | null, to_addresses: Array<EmailAddress>, cc_addresses: Array<EmailAddress>, bcc_addresses: Array<EmailAddress>, reply_to: string | null, date_sent: bigint | null, date_received: bigint, body_text: string | null, body_html: string | null, is_read: boolean, is_flagged: boolean, is_draft: boolean, is_answered: boolean, has_attachments: boolean, attachment_count: number, size_bytes: number | null, thread_id: string | null, created_at: bigint, updated_at: bigint, };


export type EmailAddress = { email: string, name: string | null, };


/**
 * Request to list emails
 */
export type ListEmailsRequest = { credential_id: bigint | null, folder_id: bigint | null, label_id: bigint | null, limit: number | null, offset: number | null, search_query: string | null, };


import type { Email } from "./Email";

/**
 * Response for email list
 */
export type ListEmailsResponse = { emails: Array<Email>, total_count: bigint, has_more: boolean, };


/**
 * Request to fetch emails by IDs
 */
export type EmailsByIdsRequest = { email_ids: Array<bigint>, };


import type { Email } from "./Email";

/**
 * Response for email batch lookup
 */
export type EmailsByIdsResponse = { emails: Array<Email>, };


export type EmailFolder = { id: bigint, credential_id: bigint, name: string, display_name: string | null, imap_path: string, folder_type: string | null, parent_folder_id: bigint | null, uidvalidity: number | null, last_synced_uid: number | null, oldest_synced_uid: number | null, total_messages: number, unread_messages: number, is_subscribed: boolean, is_selectable: boolean, created_at: bigint, updated_at: bigint, last_synced_at: bigint | null, };


export type ListFoldersRequest = { credential_id: bigint, };


import type { EmailFolder } from "./EmailFolder";

export type ListFoldersResponse = { folders: Array<EmailFolder>, };


export type EmailLabel = { id: bigint, credential_id: bigint, name: string, display_name: string | null, label_type: string, color: string | null, message_count: number, created_at: bigint, updated_at: bigint, };


export type ListLabelsRequest = { credential_id: bigint, };


import type { EmailLabel } from "./EmailLabel";

export type ListLabelsResponse = { labels: Array<EmailLabel>, };


export type Location = { id: bigint, 
/**
 * Named place, e.g. "Central Park", "Delhi Airport". Nullable for pure address locations.
 */
name: string | null, address_line1: string | null, address_line2: string | null, city: string | null, region: string | null, 
/**
 * ISO 3166-1 alpha-2 or alpha-3 country code
 */
country_code: string | null, postal_code: string | null, 
/**
 * LLM-generated summary for BM25 search during future extraction passes.
 */
search_summary: string | null, created_at: bigint, updated_at: bigint, };


import type { Location } from "./Location";

export type LocationsResponse = { locations: Array<Location>, };


export type Person = { id: bigint, email_id: bigint | null, name: string, email: string | null, phone: string | null, organisation_id: bigint | null, 
/**
 * LLM-generated summary for BM25 search during future extraction passes.
 * Should capture relational context: e.g. "engineer at Acme Corp, john@acme.com"
 */
search_summary: string | null, created_at: bigint, updated_at: bigint, };


import type { Person } from "./Person";

export type PersonsResponse = { persons: Array<Person>, };


export type DataSourceType = "email" | "imap" | "bank-statement" | "credit-card-statement" | "bank-feed" | "csv-upload" | "manual" | "unknown";


import type { BillStatus } from "./BillStatus";
import type { DataSourceType } from "./DataSourceType";
import type { TransactionCategory } from "./TransactionCategory";

/**
 * A financial document (bill, invoice, receipt, statement) extracted from an email or file.
 * One Bill may be the source for zero (unpaid) or more Transactions.
 *
 * ## Date Column Conventions
 *
 * Every date has two columns:
 * - `{field}_raw`  — `TEXT` — the exact date string as it appeared in the source document
 *                    (e.g., "15 Jan 2025", "January 15th, 2025", "15/01/25")
 * - `{field}`      — `BIGINT` — parsed UTC timestamp in milliseconds since Unix epoch.
 *                    For date-only values, use 00:00:00 UTC for that calendar day.
 *                    Nullable when parsing fails.
 */
export type Bill = { id: bigint, data_source_type: DataSourceType, data_source_id: string, status: BillStatus, category: TransactionCategory | null, 
/**
 * FK to the Organisation that issued this bill
 */
issuer_organisation_id: bigint | null, 
/**
 * FK to the Subscription this bill belongs to (if recurring)
 */
subscription_id: bigint | null, document_reference: string | null, total_amount: number | null, currency: string | null, 
/**
 * Date the bill or invoice was generated or issued by the vendor.
 * Distinct from due_date (when payment is expected) and billing_period (service window).
 * SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
 */
issued_date_raw: string | null, issued_date: bigint | null, 
/**
 * Date by which payment must be made.
 * SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
 */
due_date_raw: string | null, due_date: bigint | null, 
/**
 * Start and end of the billing or service period this bill covers.
 * SQLite column type: TEXT (raw) / BIGINT UTC ms (parsed)
 */
billing_period_start_raw: string | null, billing_period_start: bigint | null, billing_period_end_raw: string | null, billing_period_end: bigint | null, created_at: bigint, updated_at: bigint, };


export type BillStatus = "received" | "unpaid" | "paid" | "overdue" | "cancelled";


import type { ServiceIdentifierKind } from "./ServiceIdentifierKind";

export type BillSubject = { id: bigint, bill_id: bigint, kind: ServiceIdentifierKind, value: string, masked_value: string | null, is_primary: boolean, created_at: bigint, updated_at: bigint, };


export type ServiceIdentifierKind = "phone-number" | "account-number" | "policy-number" | "meter-number" | "subscription-id" | "contract-id" | "other";


import type { DataSourceType } from "./DataSourceType";
import type { TransactionStatus } from "./TransactionStatus";

/**
 * Financial transaction extracted from documents
 */
export type Transaction = { id: bigint, data_source_type: DataSourceType, data_source_id: string, amount: number, currency: string, transaction_date_raw: string | null, transaction_date: bigint | null, status: TransactionStatus, payer_organisation_id: bigint | null, payee_organisation_id: bigint | null, transaction_reference: string | null, bill_id: bigint | null, source_file: string | null, extracted_at: bigint, };


export type TransactionCategory = "income" | "expense" | "investment" | "tax" | "utility" | "subscription" | "entertainment" | "travel" | "healthcare" | "education" | "other";


export type TransactionStatus = "paid" | "cancelled" | "refunded";


import type { BillingCycle } from "./BillingCycle";

/**
 * A recurring subscription to a service extracted from emails
 */
export type Subscription = { id: bigint, 
/**
 * FK to the Organisation providing the service
 */
organisation_id: bigint | null, 
/**
 * Human-readable service name, e.g. 'Netflix', 'GitHub Pro', 'AWS'
 */
service_name: string, 
/**
 * Membership or plan tier, e.g. 'Premium', 'Pro', 'Family', 'Standard'
 */
plan_name: string | null, billing_cycle: BillingCycle | null, 
/**
 * Recurring charge amount
 */
amount: number | null, currency: string | null, 
/**
 * Raw date string exactly as it appeared in the email
 */
next_billing_date_raw: string | null, 
/**
 * Parsed UTC timestamp in milliseconds
 */
next_billing_date: bigint | null, 
/**
 * Raw date string exactly as it appeared in the email
 */
start_date_raw: string | null, 
/**
 * Parsed UTC timestamp in milliseconds
 */
start_date: bigint | null, 
/**
 * FK to the email this subscription was extracted from.
 */
source_email_id: bigint | null, created_at: bigint, updated_at: bigint, };


export type BillingCycle = "weekly" | "monthly" | "quarterly" | "semi_annual" | "annual" | "other";


import type { Subscription } from "./Subscription";

export type SubscriptionsResponse = { subscriptions: Array<Subscription>, };


import type { OrderItem } from "./OrderItem";
import type { OrderStatus } from "./OrderStatus";

/**
 * A product or e-commerce order extracted from emails
 */
export type Order = { id: bigint, 
/**
 * FK to the Organisation that is the seller/merchant
 */
organisation_id: bigint | null, order_reference: string | null, 
/**
 * Raw date string exactly as it appeared in the email
 */
order_date_raw: string | null, 
/**
 * Parsed UTC timestamp in milliseconds
 */
order_date: bigint | null, status: OrderStatus | null, total_amount: number | null, currency: string | null, items: Array<OrderItem>, tracking_number: string | null, 
/**
 * FK to the Transaction that paid for this order
 */
transaction_id: bigint | null, 
/**
 * FK to the email this order was extracted from.
 */
source_email_id: bigint | null, created_at: bigint, updated_at: bigint, };


/**
 * A single line item within an order.
 */
export type OrderItem = { name: string, quantity: number | null, unit_price: number | null, };


export type OrderStatus = "placed" | "confirmed" | "shipped" | "out_for_delivery" | "delivered" | "cancelled" | "returned" | "refunded" | "unknown";


import type { Order } from "./Order";

export type OrdersResponse = { orders: Array<Order>, };


/**
 * Financial summary/overview
 */
export type FinancialSummary = { total_income: number, total_expenses: number, net_balance: number, pending_bills: number, overdue_payments: number, currency: string, period_start: string, period_end: string, };


/**
 * Financial extraction source summary
 */
export type FinancialExtractionSummary = { source_count: bigint, transaction_count: bigint, last_extracted_at: bigint | null, };


import type { Bill } from "./Bill";
import type { CategoryBreakdown } from "./CategoryBreakdown";
import type { FinancialSummary } from "./FinancialSummary";
import type { Transaction } from "./Transaction";

/**
 * Financial health metrics
 */
export type FinancialHealth = { summary: FinancialSummary, recent_transactions: Array<Transaction>, upcoming_bills: Array<Bill>, category_breakdown: Array<CategoryBreakdown>, };


import type { TransactionCategory } from "./TransactionCategory";

/**
 * Breakdown by category
 */
export type CategoryBreakdown = { category: TransactionCategory, amount: number, percentage: number, transaction_count: number, };


export type FinancialPagination = { page: number, limit: number, total_count: number, total_pages: number, };


import type { Bill } from "./Bill";
import type { FinancialPagination } from "./FinancialPagination";

export type ListFinancialBillsResponse = { bills: Array<Bill>, pagination: FinancialPagination, };


export type FinancialTemplateType = "bill" | "transaction";


export type FinancialTemplateStatus = "active" | "superseded" | "disabled";


import type { DataSourceType } from "./DataSourceType";
import type { FinancialTemplateStatus } from "./FinancialTemplateStatus";
import type { FinancialTemplateType } from "./FinancialTemplateType";

export type FinancialExtractionTemplate = { id: bigint, data_source_type: DataSourceType, data_source_id: string, template_type: FinancialTemplateType, template_body: string, status: FinancialTemplateStatus, version: number, created_at: bigint, updated_at: bigint, };


export type FinancialTemplateVariable = { id: bigint, template_id: bigint, placeholder_name: string, target_field: string, created_at: bigint, };


import type { DataSourceType } from "./DataSourceType";

export type FinancialTemplateApplicability = { id: bigint, template_id: bigint, data_source_type: DataSourceType, data_source_id: string, match_score: number | null, created_at: bigint, };


export type DetectFinancialTemplatesRequest = { credential_id: bigint | null, max_candidate_emails: number | null, max_templates_per_sender: number | null, };


export type DetectedFinancialTemplateVariable = { placeholder_name: string, target_field: string, };


import type { DetectedFinancialTemplateVariable } from "./DetectedFinancialTemplateVariable";
import type { FinancialTemplateType } from "./FinancialTemplateType";

export type DetectedFinancialTemplate = { template_id: bigint, sender_email: string, template_type: FinancialTemplateType, template_body: string, translated_template_body: string, source_email_ids: Array<bigint>, variables: Array<DetectedFinancialTemplateVariable>, };


import type { DetectedFinancialTemplate } from "./DetectedFinancialTemplate";

export type DetectFinancialTemplatesResponse = { candidate_sender_count: number, candidate_email_count: number, templates: Array<DetectedFinancialTemplate>, };


export type TemplateDetectionSenderRank = { sender_email: string, rank: number, total_candidate_emails: number, recent_candidate_emails: number, latest_email_ts: bigint, max_existing_cluster_size: number, };


import type { DetectedFinancialTemplateVariable } from "./DetectedFinancialTemplateVariable";
import type { FinancialTemplateType } from "./FinancialTemplateType";

export type TemplateDetectionGeneratedTemplateDebug = { template_id: bigint | null, template_type: FinancialTemplateType | null, template_body: string, translated_template_body: string, source_email_ids: Array<bigint>, variables: Array<DetectedFinancialTemplateVariable>, has_bill: boolean, discarded_reason: string | null, };


import type { TemplateDetectionGeneratedTemplateDebug } from "./TemplateDetectionGeneratedTemplateDebug";

export type TemplateDetectionSenderDebug = { sender_email: string, rank: number, sender_candidate_count: number, existing_template_count: number, initially_matched_count: number, fresh_unmatched_count: number, pool_count: number, generated_templates: Array<TemplateDetectionGeneratedTemplateDebug>, error: string | null, skipped_reason: string | null, };


import type { TemplateDetectionSenderDebug } from "./TemplateDetectionSenderDebug";
import type { TemplateDetectionSenderRank } from "./TemplateDetectionSenderRank";

export type TemplateDetectionDebugState = { keyword_query: string, keyword_list: Array<string>, max_candidate_emails: number, matched_email_ids_count: number, sender_ranking: Array<TemplateDetectionSenderRank>, candidate_email_ids: Array<bigint>, sender_debug: Array<TemplateDetectionSenderDebug>, };


export type FinancialTemplateDetectionJobStatus = "idle" | "running" | "completed" | "failed";


import type { FinancialTemplateDetectionJobStatus } from "./FinancialTemplateDetectionJobStatus";
import type { TemplateDetectionDebugState } from "./TemplateDetectionDebugState";

export type FinancialTemplateDetectionJobState = { run_id: bigint, status: FinancialTemplateDetectionJobStatus, started_at: bigint | null, finished_at: bigint | null, total_senders: number, processed_senders: number, current_sender: string | null, candidate_sender_count: number, candidate_email_count: number, new_templates_count: number, error: string | null, debug: TemplateDetectionDebugState | null, };


export type TemplateDetectionSenderLlmDraftPreview = { seed_text: string, cluster_size: number, selected_email_ids: Array<bigint>, full_template: string, sample_subject: string, sample_body: string, };


import type { TemplateDetectionSenderLlmDraftPreview } from "./TemplateDetectionSenderLlmDraftPreview";

export type TemplateDetectionSenderLlmInputsResponse = { sender_email: string, sender_candidate_count: number, existing_template_count: number, initially_matched_count: number, fresh_unmatched_count: number, pool_count: number, drafts: Array<TemplateDetectionSenderLlmDraftPreview>, };


export type FinancialTemplateFieldMapping = { placeholder_name: string, target_field: string, };


import type { FinancialExtractionTemplate } from "./FinancialExtractionTemplate";
import type { FinancialTemplateFieldMapping } from "./FinancialTemplateFieldMapping";

export type FinancialTemplateWithVariables = { template: FinancialExtractionTemplate, variables: Array<FinancialTemplateFieldMapping>, };


import type { FinancialTemplateWithVariables } from "./FinancialTemplateWithVariables";

export type ListFinancialTemplatesResponse = { templates: Array<FinancialTemplateWithVariables>, };


export type DeleteFinancialTemplatesRequest = { template_ids: Array<bigint>, };


export type DeleteFinancialTemplatesResponse = { deleted_count: number, };
