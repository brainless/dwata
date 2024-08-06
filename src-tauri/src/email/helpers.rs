use super::{Email, EmailBucket, EmailFull};
use crate::{
    email_account::{EmailAccount, Mailbox},
    error::DwataError,
};
use chrono::DateTime;
use log::{error, info};
use mail_parser::MessageParser;
use std::{collections::HashMap, fs::create_dir_all, path::PathBuf};
use tantivy::{
    collector::TopDocs,
    directory::MmapDirectory,
    doc,
    query::{AllQuery, QueryParser},
    schema::{Facet, FacetOptions, Schema, Value, FAST, STORED, TEXT},
    Index, ReloadPolicy, TantivyDocument, TantivyError,
};

pub async fn parse_email_from_file(
    mailbox: &Mailbox,
    mail_uid: String,
    email_file: &[u8],
) -> Result<EmailFull, DwataError> {
    match MessageParser::default().parse(email_file) {
        Some(parsed) => {
            let from = parsed.from().ok_or(DwataError::CouldNotParseEmailFile)?;
            let from = from.first().ok_or(DwataError::CouldNotParseEmailFile)?;
            let from = (
                from.name()
                    .map_or_else(|| "".to_string(), |x| x.to_string()),
                from.address().unwrap().to_string(),
            );

            let date = parsed.date().ok_or(DwataError::CouldNotParseEmailFile)?;
            let date = match DateTime::parse_from_rfc3339(&date.to_rfc3339()) {
                Ok(dt) => dt.timestamp(),
                Err(_) => return Err(DwataError::CouldNotParseEmailFile),
            };

            let subject = parsed.subject().ok_or(DwataError::CouldNotParseEmailFile)?;
            let body_text = parsed
                .body_text(0)
                .ok_or(DwataError::CouldNotParseEmailFile)?;

            return Ok(EmailFull {
                email: Email {
                    uid: mail_uid.parse::<u32>().unwrap(),
                    mailbox_id: mailbox.id,
                    message_id: parsed.message_id().map(|x| x.to_string()),
                    in_reply_to: parsed.in_reply_to().as_text().map(|x| x.to_string()),
                    from_name: from.0,
                    from_email: from.1,
                    date,
                    subject: subject.to_string(),
                    ..Default::default()
                },
                body_text: body_text.to_string(),
            });
        }
        None => return Err(DwataError::CouldNotParseEmailFile),
    }
}

pub fn store_emails_to_tantity(
    email_account: &EmailAccount,
    mailbox: &Mailbox,
    emails: &Vec<EmailFull>,
    storage_dir: &PathBuf,
) -> Result<(), DwataError> {
    let mut schema_builder = Schema::builder();
    let mailbox_facet = schema_builder.add_facet_field("mailbox", FacetOptions::default());
    let uid = schema_builder.add_u64_field("uid", FAST | STORED);
    let subject = schema_builder.add_text_field("subject", TEXT | STORED);
    let body = schema_builder.add_text_field("body", TEXT);
    let from_name = schema_builder.add_text_field("from_name", TEXT | STORED);
    let from_email = schema_builder.add_text_field("from_email", TEXT | STORED);
    let schema = schema_builder.build();

    let mut storage_dir = storage_dir.clone();
    storage_dir.push("search_index");
    if !storage_dir.as_path().exists() {
        create_dir_all(storage_dir.as_path()).unwrap();
    }
    let mmap_dir = match MmapDirectory::open(&storage_dir) {
        Ok(mmap_dir) => mmap_dir,
        Err(err) => {
            error!("Could not open mmap directory\nError: {}", err);
            return Err(DwataError::CouldNotCreateSearchIndex);
        }
    };
    let index = match Index::open_or_create(mmap_dir, schema.clone()) {
        Ok(index) => index,
        Err(err) => {
            match &err {
                TantivyError::SchemaError(err_string) => {
                    if err_string == "An index exists but the schema does not match." {
                        // We delete the index and try again
                        std::fs::remove_dir_all(storage_dir.as_path()).unwrap();
                        if !storage_dir.as_path().exists() {
                            create_dir_all(storage_dir.as_path()).unwrap();
                        }
                        Index::create_in_dir(&storage_dir, schema.clone()).unwrap()
                    } else {
                        error!("Could not open or create search index\nError: {}", err);
                        return Err(DwataError::CouldNotCreateSearchIndex);
                    }
                }
                _ => {
                    error!("Could not open or create search index\nError: {}", err);
                    return Err(DwataError::CouldNotCreateSearchIndex);
                }
            }
        }
    };
    let mut index_writer = index.writer(100_000_000).unwrap();
    for email_full in emails {
        index_writer
            .add_document(doc!(
                mailbox_facet => Facet::from(&format!("/{}/{}", email_account.email_address, mailbox.name)),
                uid => u64::from(email_full.email.uid),
                subject => email_full.email.subject.clone(),
                body => email_full.body_text.clone(),
                from_name => email_full.email.from_name.clone(),
                from_email => email_full.email.from_email.clone()
            ))
            .unwrap();
    }
    index_writer.commit().unwrap();
    Ok(())
}

pub async fn search_emails_from_tantivy(
    search_query: Option<String>,
    _email_account: &EmailAccount,
    _mailbox: &Mailbox,
    storage_dir: &PathBuf,
) -> Result<Vec<Email>, DwataError> {
    let mut storage_dir = storage_dir.clone();
    storage_dir.push("search_index");
    if !storage_dir.as_path().exists() {
        error!("Search index directory does not exist");
        return Err(DwataError::SearchIndexDoesNotExist);
    }
    let mmap_dir = match MmapDirectory::open(&storage_dir) {
        Ok(mmap_dir) => mmap_dir,
        Err(err) => {
            error!("Could not open mmap directory\nError: {}", err);
            return Err(DwataError::CouldNotCreateSearchIndex);
        }
    };
    let index = match Index::open(mmap_dir) {
        Ok(index) => index,
        Err(err) => {
            error!("Could not open search index\nError: {}", err);
            return Err(DwataError::CouldNotOpenSearchIndex);
        }
    };
    let reader = match index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()
    {
        Ok(reader) => reader,
        Err(err) => {
            error!("Could not open search index reader\nError: {}", err);
            return Err(DwataError::CouldNotOpenSearchIndex);
        }
    };
    let searcher = reader.searcher();
    let mut schema_builder = Schema::builder();
    let mailbox_facet = schema_builder.add_facet_field("mailbox", FacetOptions::default());
    let uid = schema_builder.add_u64_field("uid", FAST | STORED);
    let subject = schema_builder.add_text_field("subject", TEXT | STORED);
    let body = schema_builder.add_text_field("body", TEXT);
    let from_name = schema_builder.add_text_field("from_name", TEXT | STORED);
    let from_email = schema_builder.add_text_field("from_email", TEXT | STORED);
    let schema = schema_builder.build();

    let query = match search_query {
        Some(query) => {
            let query_parser = QueryParser::for_index(&index, vec![subject, body]);
            info!("Search query {}", query);
            match query_parser.parse_query(&query) {
                Ok(query) => query,
                Err(err) => {
                    error!("Could not parse query\nError: {}", err);
                    return Err(DwataError::CouldNotParseSearchQuery);
                }
            }
        }
        None => Box::new(AllQuery),
    };
    let top_docs = match searcher.search(&query, &TopDocs::with_limit(100)) {
        Ok(top_docs) => top_docs,
        Err(err) => {
            error!("Could not search index\nError: {}", err);
            return Err(DwataError::CouldNotSearchTheIndex);
        }
    };
    let mut emails: Vec<Email> = vec![];
    for (_score, doc_address) in top_docs {
        match searcher.doc::<TantivyDocument>(doc_address) {
            Ok(doc) => {
                emails.push(Email {
                    uid: doc.get_first(uid).unwrap().as_u64().unwrap() as u32,
                    mailbox_id: doc.get_first(uid).unwrap().as_u64().unwrap() as i64,
                    from_name: doc
                        .get_first(from_name)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    from_email: doc
                        .get_first(from_email)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    // date: doc.get_first("date").unwrap().as_i64().unwrap(),
                    subject: doc
                        .get_first(subject)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    ..Default::default()
                });
            }
            Err(err) => {
                error!("Could not retrieve document\nError: {}", err);
                continue;
            }
        };
    }
    Ok(emails)
}

async fn find_and_index_email_buckets(emails: &Vec<Email>) -> Vec<EmailBucket> {
    // We create buckets of emails that are similar or are in a thread or belong to same sender, etc.
    let mut buckets: Vec<EmailBucket> = vec![];

    for email in emails.iter() {
        // We are checking if this email is a reply to another email
        // We need to do this recursively, so we keep checking the parent email until we find the root email
        // which does not have an `in_reply_to` field
        info!("Email: {}: {}", email.uid, email.subject);
        let mut root_email = email;
        let mut found_in_reply_to = false;
        while let Some(in_reply_to) = root_email.in_reply_to.as_ref() {
            found_in_reply_to = true;
            root_email = match emails
                .iter()
                .filter(|item| item.message_id.as_ref().is_some_and(|x| x == in_reply_to))
                .nth(0)
            {
                Some(x) => x,
                None => break,
            };
        }
        info!(
            "Found root email: {}: {} - {}",
            root_email.uid,
            root_email
                .in_reply_to
                .as_ref()
                .unwrap_or(&"None".to_string()),
            root_email.subject
        );

        if found_in_reply_to {
            // This email is a reply to another email, let's find the root email recursively
            // Let's find the bucket for this email
            if let Some(bucket) = buckets
                .iter_mut()
                .find(|x| x.root_email_uid == Some(root_email.uid))
            {
                // We found the bucket, let's add this email to it
                bucket.uid_list.push(email.uid);
            } else {
                // We did not find the bucket, let's create a new one
                let mut clean_subject = root_email.subject.clone().trim().to_string();
                // Remove "Re: " from the beginning of the subject
                if clean_subject.starts_with("Re: ") {
                    clean_subject = clean_subject.replace("Re: ", "");
                }
                if clean_subject.is_empty() {
                    continue;
                }
                buckets.push(EmailBucket {
                    root_email_uid: Some(root_email.uid),
                    uid_list: vec![email.uid],
                    summary: Some(clean_subject),
                    ..Default::default()
                });
            }
        };
    }

    // Let's group the emails by subject, keeping a vector of IDs for each email as value of the HashMap
    let mut subject_buckets: HashMap<String, Vec<u32>> = HashMap::new();
    for email in emails {
        let mut clean_subject = email.subject.clone().trim().to_string();
        // Remove "Re: " from the beginning of the subject
        if clean_subject.starts_with("Re: ") {
            clean_subject = clean_subject.replace("Re: ", "");
        }
        if clean_subject.is_empty() {
            continue;
        }
        subject_buckets
            .entry(clean_subject)
            .and_modify(|x| x.push(email.uid))
            .or_insert(vec![email.uid]);
    }

    // If we have subject buckets with count > 1, then we create a new bucket for each subject
    // for (subject, vector) in subject_buckets {
    //     if vector.len() > 1 {
    //         buckets.push(EmailBucket {
    //             root_email_id: None,
    //             id_list_raw: vector,
    //             summary: Some(subject.clone()),
    //             ..Default::default()
    //         });
    //     }
    // }

    info!(
        "Found {} buckets\n Subjects: {}",
        buckets.len(),
        buckets
            .iter()
            .map(|x| format!(
                "{}: {}",
                x.root_email_uid.as_ref().unwrap(),
                x.summary.clone().unwrap()
            ))
            .collect::<Vec<String>>()
            .join("\n")
    );
    buckets
}
