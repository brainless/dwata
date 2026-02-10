use anyhow::{bail, Result};
use regex::Regex;
use std::time::Duration;

pub struct PatternValidationError {
    pub field: String,
    pub message: String,
}

pub fn validate_pattern(
    name: &str,
    regex_pattern: &str,
    amount_group: usize,
    vendor_group: Option<usize>,
    source_vendor_group: Option<usize>,
    destination_vendor_group: Option<usize>,
    date_group: Option<usize>,
    reference_group: Option<usize>,
    currency_group: Option<usize>,
    document_type: &str,
    status: &str,
) -> Result<()> {
    validate_pattern_name(name)?;

    let regex = validate_regex_compiles(regex_pattern)?;

    if vendor_group.is_some() {
        bail!("vendor_group is deprecated; use source_vendor_group/destination_vendor_group");
    }

    if source_vendor_group.is_none() && destination_vendor_group.is_none() {
        bail!("At least one of source_vendor_group or destination_vendor_group is required");
    }

    validate_capture_groups(
        &regex,
        amount_group,
        vendor_group,
        source_vendor_group,
        destination_vendor_group,
        date_group,
        reference_group,
        currency_group,
    )?;

    validate_document_type(document_type)?;

    validate_status(status)?;

    validate_regex_performance(&regex)?;

    Ok(())
}

fn validate_pattern_name(name: &str) -> Result<()> {
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        bail!("Pattern name must contain only lowercase letters, numbers, and underscores");
    }
    if name.is_empty() || name.len() > 100 {
        bail!("Pattern name must be 1-100 characters");
    }
    Ok(())
}

fn validate_regex_compiles(pattern: &str) -> Result<Regex> {
    Regex::new(pattern).map_err(|e| anyhow::anyhow!("Invalid regex pattern: {}", e))
}

fn validate_capture_groups(
    regex: &Regex,
    amount_group: usize,
    vendor_group: Option<usize>,
    source_vendor_group: Option<usize>,
    destination_vendor_group: Option<usize>,
    date_group: Option<usize>,
    reference_group: Option<usize>,
    currency_group: Option<usize>,
) -> Result<()> {
    let group_count = regex.captures_len();

    if amount_group >= group_count {
        bail!(
            "amount_group {} does not exist (regex has {} groups)",
            amount_group,
            group_count
        );
    }

    if let Some(vg) = vendor_group {
        if vg >= group_count {
            bail!(
                "vendor_group {} does not exist (regex has {} groups)",
                vg,
                group_count
            );
        }
    }

    if let Some(vg) = source_vendor_group {
        if vg >= group_count {
            bail!(
                "source_vendor_group {} does not exist (regex has {} groups)",
                vg,
                group_count
            );
        }
    }

    if let Some(vg) = destination_vendor_group {
        if vg >= group_count {
            bail!(
                "destination_vendor_group {} does not exist (regex has {} groups)",
                vg,
                group_count
            );
        }
    }

    if let Some(dg) = date_group {
        if dg >= group_count {
            bail!(
                "date_group {} does not exist (regex has {} groups)",
                dg,
                group_count
            );
        }
    }

    if let Some(rg) = reference_group {
        if rg >= group_count {
            bail!(
                "reference_group {} does not exist (regex has {} groups)",
                rg,
                group_count
            );
        }
    }

    if let Some(cg) = currency_group {
        if cg >= group_count {
            bail!(
                "currency_group {} does not exist (regex has {} groups)",
                cg,
                group_count
            );
        }
    }

    Ok(())
}

fn validate_document_type(doc_type: &str) -> Result<()> {
    let valid_types = [
        "invoice",
        "bill",
        "receipt",
        "payment-confirmation",
        "other",
    ];
    if !valid_types.contains(&doc_type) {
        bail!("Invalid document_type. Must be one of: {:?}", valid_types);
    }
    Ok(())
}

fn validate_status(status: &str) -> Result<()> {
    let valid_statuses = ["paid", "pending", "overdue", "cancelled"];
    if !valid_statuses.contains(&status) {
        bail!("Invalid status. Must be one of: {:?}", valid_statuses);
    }
    Ok(())
}

fn validate_regex_performance(regex: &Regex) -> Result<()> {
    let test_input = "a".repeat(1000);

    let start = std::time::Instant::now();
    let _ = regex.is_match(&test_input);
    let duration = start.elapsed();

    if duration > Duration::from_millis(100) {
        bail!("Regex pattern is too slow (potential catastrophic backtracking)");
    }

    Ok(())
}
