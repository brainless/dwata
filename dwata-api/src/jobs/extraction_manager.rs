use crate::database::companies as companies_db;
use crate::database::contact_links as contact_links_db;
use crate::database::contacts as contacts_db;
use crate::database::emails as emails_db;
use crate::database::extraction_jobs as jobs_db;
use crate::database::events as events_db;
use crate::database::linkedin_connections as linkedin_connections_db;
use crate::database::positions as positions_db;
use crate::database::AsyncDbConnection;
use anyhow::Result;
use chrono::NaiveDate;
use extractors::linkedin_archive::LinkedInArchiveExtractor;
use shared_types::{ArchiveType, DataType, ExtractionJobStatus, ExtractionSourceType, ExtractorType, extraction::{ExtractionInput, ExtractedEntity, Attachment}, EmailAttachment};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::sync::Mutex;

pub struct ExtractionManager {
    db_conn: AsyncDbConnection,
    active_jobs: Arc<Mutex<HashMap<i64, JoinHandle<()>>>>,
}

impl ExtractionManager {
    pub fn new(db_conn: AsyncDbConnection) -> Self {
        Self {
            db_conn,
            active_jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start_job(&self, job_id: i64) -> Result<()> {
        let job = jobs_db::get_extraction_job(self.db_conn.clone(), job_id).await?;

        let active_jobs = self.active_jobs.lock().await;
        if active_jobs.contains_key(&job_id) {
            drop(active_jobs);
            return Err(anyhow::anyhow!("Job already running"));
        }
        drop(active_jobs);

        jobs_db::update_job_status(
            self.db_conn.clone(),
            job_id,
            ExtractionJobStatus::Running,
            None,
        )
        .await?;

        let db_conn = self.db_conn.clone();
        let job = job.clone();
        let active_jobs = self.active_jobs.clone();

        let handle = tokio::spawn(async move {
            let result = match (job.source_type.clone(), job.extractor_type.clone()) {
                (ExtractionSourceType::EmailAttachment, ExtractorType::AttachmentParser) => {
                    Self::run_extraction_job(&db_conn, job_id).await
                }
                (ExtractionSourceType::LocalArchive, ExtractorType::LinkedInArchive) => {
                    Self::run_linkedin_extraction_job(&db_conn, job_id).await
                }
                _ => Err(anyhow::anyhow!("Unsupported extraction type combination")),
            };

            let status = match result {
                Ok(_) => ExtractionJobStatus::Completed,
                Err(_) => ExtractionJobStatus::Failed,
            };

            let _ = jobs_db::update_job_status(db_conn.clone(), job_id, status, None).await;

            let mut jobs = active_jobs.lock().await;
            jobs.remove(&job_id);
        });

        let mut jobs = self.active_jobs.lock().await;
        jobs.insert(job_id, handle);

        Ok(())
    }

    async fn run_extraction_job(
        db_conn: &AsyncDbConnection,
        job_id: i64,
    ) -> Result<()> {
        let job = jobs_db::get_extraction_job(db_conn.clone(), job_id).await?;

        match &job.source_type {
            ExtractionSourceType::EmailAttachment => {
                let source_config: shared_types::extraction_job::ExtractionSourceConfig = serde_json::from_value(job.source_config.clone())?;

                if let shared_types::extraction_job::ExtractionSourceConfig::EmailAttachments { email_ids, attachment_types, status_filter: _ } = source_config {
                    let mut attachments = Vec::new();

                    if let Some(ids) = email_ids {
                        for email_id in ids {
                            let atts = emails_db::get_email_attachments(db_conn.clone(), email_id).await?;
                            attachments.extend(atts);
                        }
                    }

                    let types = attachment_types.clone();

                    extract_from_email_attachments(
                        db_conn.clone(),
                        job_id,
                        attachments,
                        types,
                    )
                    .await?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    async fn run_linkedin_extraction_job(
        db_conn: &AsyncDbConnection,
        job_id: i64,
    ) -> Result<()> {
        let job = jobs_db::get_extraction_job(db_conn.clone(), job_id).await?;

        let source_config: shared_types::extraction_job::ExtractionSourceConfig = serde_json::from_value(job.source_config.clone())?;

        if let shared_types::extraction_job::ExtractionSourceConfig::LocalArchive { archive_path, archive_type, files_to_process } = source_config {
            extract_from_local_archive(
                db_conn.clone(),
                job_id,
                archive_path,
                archive_type,
                files_to_process,
            )
            .await?;
        }

        Ok(())
    }
}

// Mock extractor for email attachments
struct AttachmentParserExtractor;

impl AttachmentParserExtractor {
    fn with_defaults() -> Self {
        Self
    }

    fn extract(&self, _input: &ExtractionInput) -> Result<Vec<shared_types::extraction::ExtractionResult>, shared_types::extraction::ExtractionError> {
        // Placeholder - would normally parse attachments
        Ok(vec![])
    }
}

async fn extract_from_email_attachments(
    db_conn: AsyncDbConnection,
    job_id: i64,
    attachments: Vec<EmailAttachment>,
    attachment_types: Vec<String>,
) -> Result<()> {

        let filtered_attachments: Vec<_> = attachments
            .into_iter()
            .filter(|att| {
                if let Some(content_type) = &att.content_type {
                    attachment_types.iter().any(|t| content_type.contains(t))
                } else {
                    false
                }
            })
            .collect();

        jobs_db::update_job_progress(
            db_conn.clone(),
            job_id,
            Some(filtered_attachments.len() as u64),
            None,
            None,
            None,
            None,
        )
        .await?;

        let extractor = AttachmentParserExtractor::with_defaults();

        let mut processed = 0;
        let mut events_count = 0;
        let mut contacts_count = 0;

        for attachment in filtered_attachments {
            let content = std::fs::read(&attachment.file_path).unwrap_or_default();
            let content_type = attachment.content_type.clone().unwrap_or_default();

            let input = ExtractionInput {
                email_id: format!("email_{}", attachment.email_id),
                subject: String::new(),
                body_text: String::new(),
                body_html: None,
                attachments: vec![Attachment {
                    filename: attachment.filename.clone(),
                    content_type: content_type.clone(),
                    content,
                }],
                sender: shared_types::extraction::EmailAddress {
                    email: String::new(),
                    name: None,
                },
                recipients: vec![],
                timestamp: chrono::Utc::now().timestamp(),
                thread_id: None,
                in_reply_to: None,
                extracted_entities: vec![],
                existing_projects: vec![],
                existing_tasks: vec![],
                existing_contacts: vec![],
                user_timezone: "UTC".to_string(),
                user_language: "en".to_string(),
                user_preferences: shared_types::extraction::UserPreferences {
                    date_format: "YYYY-MM-DD".to_string(),
                    default_task_priority: shared_types::TaskPriority::Medium,
                    default_project_status: shared_types::ProjectStatus::Active,
                    auto_link_threshold: 0.8,
                },
                target_data_type: DataType::Event,
                min_confidence: 0.7,
                max_results: None,
            };

            match extractor.extract(&input) {
                Ok(results) => {
                    for result in results {
                        match result.entity {
                            ExtractedEntity::Event(event) => {
                                let event_date = chrono::DateTime::parse_from_rfc3339(&event.date)
                                    .map(|dt| dt.timestamp())
                                    .unwrap_or_else(|_| chrono::Utc::now().timestamp());

                                if events_db::insert_event_from_extraction(
                                    db_conn.clone(),
                                    job_id,
                                    Some(attachment.email_id),
                                    event.name,
                                    event.description,
                                    event_date,
                                    event.location,
                                    event.attendees,
                                    result.confidence,
                                    result.requires_review,
                                )
                                .await
                                .is_ok()
                                {
                                    events_count += 1;
                                }
                            }
                            ExtractedEntity::Contact(contact) => {
                                match contacts_db::insert_contact_from_extraction(
                                    db_conn.clone(),
                                    job_id,
                                    Some(attachment.email_id),
                                    contact.name,
                                    contact.email,
                                    contact.phone,
                                    contact.organization,
                                    result.confidence,
                                    result.requires_review,
                                )
                                .await
                                {
                                    Ok(_) => contacts_count += 1,
                                    Err(e) => {
                                        tracing::warn!("Skipping duplicate contact: {}", e);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    emails_db::update_attachment_extraction_status(
                        db_conn.clone(),
                        attachment.id,
                        "completed",
                    )
                    .await?;
                }
                Err(e) => {
                    tracing::error!("Failed to extract from attachment {}: {}", attachment.id, e);

                    emails_db::update_attachment_extraction_status(
                        db_conn.clone(),
                        attachment.id,
                        "failed",
                    )
                    .await?;
                }
            }

            processed += 1;

            jobs_db::update_job_progress(
                db_conn.clone(),
                job_id,
                None,
                Some(processed),
                Some(events_count),
                Some(contacts_count),
                None,
            )
            .await?;
        }

        Ok(())
    }

async fn extract_from_local_file(
        _db_conn: AsyncDbConnection,
        _job_id: i64,
        _file_path: String,
        _content_type: String,
    ) -> Result<()> {
        tracing::warn!("Local file extraction not yet implemented");
        Ok(())
    }

async fn extract_from_local_archive(
        db_conn: AsyncDbConnection,
        job_id: i64,
        archive_path: String,
        archive_type: ArchiveType,
        files_to_process: Vec<String>,
    ) -> Result<()> {
        tracing::info!("Extracting from local archive: {} ({:?})", archive_path, archive_type);

        if !matches!(archive_type, ArchiveType::LinkedIn) {
            return Err(anyhow::anyhow!("Unsupported archive type: {:?}", archive_type));
        }

        let extractor = LinkedInArchiveExtractor::new();

        let results = extractor.process_archive(&archive_path, &files_to_process)?;

        let total_items = results.len() as u64;

        jobs_db::update_job_progress(
            db_conn.clone(),
            job_id,
            Some(total_items),
            None,
            None,
            None,
            None,
        )
        .await?;

        let mut processed = 0;
        let mut contacts_count = 0;
        let mut companies_count = 0;
        let mut positions_count = 0;

        let mut contact_map: HashMap<String, i64> = HashMap::new();
        let mut company_map: HashMap<String, i64> = HashMap::new();

        for result in results {
            match result.entity {
                ExtractedEntity::Contact(extracted_contact) => {
                    let contact_id = match contacts_db::insert_contact_from_extraction(
                        db_conn.clone(),
                        job_id,
                        None,
                        extracted_contact.name.clone(),
                        extracted_contact.email.clone(),
                        extracted_contact.phone,
                        extracted_contact.organization,
                        result.confidence,
                        result.requires_review,
                    )
                    .await
                    {
                        Ok(id) => {
                            contacts_count += 1;
                            id
                        }
                        Err(e) => {
                            tracing::warn!("Skipping duplicate contact: {}", e);
                            if let Some(email) = &extracted_contact.email {
                                if let Ok(id) = get_contact_id_by_email(db_conn.clone(), email).await {
                                    id
                                } else {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                        }
                    };

                    contact_map.insert(extracted_contact.name.clone(), contact_id);

                    for profile_url in extracted_contact.profile_urls {
                        let link_type = match profile_url.link_type.as_str() {
                            "linkedin" => shared_types::ContactLinkType::Linkedin,
                            "github" => shared_types::ContactLinkType::Github,
                            "twitter" => shared_types::ContactLinkType::Twitter,
                            "personal" => shared_types::ContactLinkType::Personal,
                            _ => shared_types::ContactLinkType::Other,
                        };

                        let _ = contact_links_db::insert_contact_link(
                            db_conn.clone(),
                            contact_id,
                            link_type,
                            profile_url.url,
                            None,
                            false,
                        )
                        .await;
                    }
                }
                ExtractedEntity::Company(extracted_company) => {
                    let company_id = companies_db::get_or_create_company(
                        db_conn.clone(),
                        Some(job_id),
                        extracted_company.name.clone(),
                        extracted_company.location,
                    )
                    .await?;

                    company_map.insert(extracted_company.name.clone(), company_id);
                    companies_count += 1;
                }
                ExtractedEntity::Position(extracted_position) => {
                    let company_id = *company_map
                        .get(&extracted_position.company_name)
                        .ok_or_else(|| anyhow::anyhow!("Company not found: {}", extracted_position.company_name))?;

                    let contact_id = *contact_map
                        .get(&extracted_position.contact_name)
                        .unwrap_or(&0);

                    if contact_id == 0 {
                        continue;
                    }

                    let started_date = extracted_position.started_on.as_ref().and_then(|d| parse_linkedin_date(d));
                    let finished_date = extracted_position.finished_on.as_ref().and_then(|d| parse_linkedin_date(d));

                    let _ = positions_db::insert_position(
                        db_conn.clone(),
                        Some(job_id),
                        contact_id,
                        company_id,
                        extracted_position.title,
                        extracted_position.description,
                        extracted_position.location,
                        extracted_position.started_on,
                        extracted_position.finished_on,
                        started_date,
                        finished_date,
                        extracted_position.is_current,
                        None,
                        false,
                    )
                    .await;

                    positions_count += 1;
                }
                _ => {}
            }

            processed += 1;

            jobs_db::update_job_progress(
                db_conn.clone(),
                job_id,
                None,
                Some(processed),
                Some(companies_count),
                Some(positions_count),
                Some(contacts_count),
            )
            .await?;
        }

        let connections_metadata = extractor.extract_connections_metadata(&archive_path)?;

        for metadata in connections_metadata {
            let full_name = format!("{} {}", metadata.first_name, metadata.last_name);
            let contact_id = if let Some(id) = contact_map.get(&full_name) {
                *id
            } else if let Some(email) = &metadata.email_address {
                if let Ok(id) = get_contact_id_by_email(db_conn.clone(), email).await {
                    id
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let connected_date = parse_linkedin_date(&metadata.connected_on);

            let _ = linkedin_connections_db::insert_linkedin_connection(
                db_conn.clone(),
                job_id,
                contact_id,
                Some(metadata.connected_on),
                connected_date,
                "connections".to_string(),
                None,
                None,
                None,
                metadata.company_at_connection,
                metadata.position_at_connection,
            )
            .await;
        }

        let invitations_metadata = extractor.extract_invitations_metadata(&archive_path)?;

        for metadata in invitations_metadata {
            let (contact_name, contact_url, invitation_message, invitation_sent_at) =
                if metadata.direction == "OUTGOING" {
                    (metadata.to.clone(), metadata.invitee_profile_url.clone(), metadata.message.clone(), Some(metadata.sent_at.clone()))
                } else {
                    (metadata.from.clone(), metadata.inviter_profile_url.clone(), metadata.message.clone(), Some(metadata.sent_at.clone()))
                };

            let contact_id = if let Some(id) = contact_map.get(&contact_name) {
                *id
            } else {
                continue;
            };

            let _ = linkedin_connections_db::insert_linkedin_connection(
                db_conn.clone(),
                job_id,
                contact_id,
                None,
                None,
                "invitations".to_string(),
                Some(metadata.direction.clone()),
                invitation_message,
                invitation_sent_at,
                None,
                None,
            )
            .await;

            if let Some(url) = contact_url {
                let link_type = shared_types::ContactLinkType::Linkedin;
                let _ = contact_links_db::insert_contact_link(
                    db_conn.clone(),
                    contact_id,
                    link_type,
                    url,
                    None,
                    false,
                )
                .await;
            }
        }

        Ok(())
    }

async fn get_contact_id_by_email(db_conn: AsyncDbConnection, email: &str) -> Result<i64> {
        let conn = db_conn.lock().await;
        let id: i64 = conn.query_row(
            "SELECT id FROM contacts WHERE email = ? LIMIT 1",
            [email],
            |row| row.get(0),
        )?;
        Ok(id)
    }

fn parse_linkedin_date(date_str: &str) -> Option<i64> {
        if date_str.trim().is_empty() {
            return None;
        }

        if let Ok(date) = NaiveDate::parse_from_str(date_str, "%d %b %Y") {
            return Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp());
        }

        if let Ok(date) = NaiveDate::parse_from_str(&format!("01 {}", date_str), "%d %b %Y") {
            return Some(date.and_hms_opt(0, 0, 0)?.and_utc().timestamp());
        }

        None
    }
