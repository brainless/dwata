use crate::database::extraction_jobs as jobs_db;
use crate::database::events as events_db;
use crate::database::contacts as contacts_db;
use crate::database::emails as emails_db;
use crate::database::AsyncDbConnection;
use anyhow::Result;
use extractors::{AttachmentParserExtractor, Extractor};
use shared_types::extraction::{DataType, ExtractionInput, ExtractedEntity};
use shared_types::extraction_job::{ExtractionJob, ExtractionJobStatus, ExtractionSourceConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

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

        let handle = tokio::spawn(async move {
            if let Err(e) = Self::run_extraction(db_conn.clone(), &job).await {
                tracing::error!("Extraction failed for job {}: {}", job_id, e);
                let _ = jobs_db::update_job_status(
                    db_conn.clone(),
                    job_id,
                    ExtractionJobStatus::Failed,
                    Some(e.to_string()),
                )
                .await;
            }
        });

        let mut active_jobs = self.active_jobs.lock().await;
        active_jobs.insert(job_id, handle);

        Ok(())
    }

    async fn run_extraction(db_conn: AsyncDbConnection, job: &ExtractionJob) -> Result<()> {
        tracing::info!("Starting extraction for job {}", job.id);

        let config: ExtractionSourceConfig = serde_json::from_value(job.source_config.clone())?;

        match config {
            ExtractionSourceConfig::EmailAttachments {
                email_ids,
                attachment_types,
                status_filter,
            } => {
                Self::extract_from_email_attachments(
                    db_conn.clone(),
                    job.id,
                    email_ids,
                    attachment_types,
                    status_filter,
                )
                .await?;
            }
            ExtractionSourceConfig::LocalFile {
                file_path,
                content_type,
            } => {
                Self::extract_from_local_file(db_conn.clone(), job.id, file_path, content_type).await?;
            }
        }

        jobs_db::update_job_status(
            db_conn.clone(),
            job.id,
            ExtractionJobStatus::Completed,
            None,
        )
        .await?;

        tracing::info!("Extraction completed for job {}", job.id);
        Ok(())
    }

    async fn extract_from_email_attachments(
        db_conn: AsyncDbConnection,
        job_id: i64,
        email_ids: Option<Vec<i64>>,
        attachment_types: Vec<String>,
        _status_filter: shared_types::extraction_job::AttachmentExtractionFilter,
    ) -> Result<()> {
        let attachments = if let Some(ids) = email_ids {
            let mut all_attachments = Vec::new();
            for email_id in ids {
                let email_attachments = emails_db::get_email_attachments(db_conn.clone(), email_id).await?;
                all_attachments.extend(email_attachments);
            }
            all_attachments
        } else {
            emails_db::list_pending_attachments(db_conn.clone(), 1000).await?
        };

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
            let content_type = attachment.content_type.unwrap_or_default();

            let input = ExtractionInput {
                email_id: format!("email_{}", attachment.email_id),
                subject: String::new(),
                body_text: String::new(),
                body_html: None,
                attachments: vec![shared_types::extraction::Attachment {
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
}
