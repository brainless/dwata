use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use dwata_agents::statement_extractor::{
    build_template, extract_transactions, infer_currency_from_intro, infer_field_mapping,
    read_statement_sheets,
};

#[derive(Parser, Debug)]
#[command(
    name = "statement-based-financial-extractor",
    about = "Extract bank statement rows from local XLSX files into typed transaction rows."
)]
struct Cli {
    #[arg(long, required = true)]
    input: PathBuf,

    #[arg(long)]
    sheet: Option<String>,

    #[arg(long, default_value_t = false)]
    template_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let sheets = read_statement_sheets(&cli.input, cli.sheet.as_deref())?;
    if sheets.is_empty() {
        return Err(anyhow::anyhow!(
            "No columnar statement table found in {}",
            cli.input.display()
        ));
    }

    for sheet in sheets {
        let template = build_template(&sheet);
        println!("\n=== Sheet: {} ===", sheet.name);
        println!("Rows detected: {}", sheet.rows.len());
        println!("Headers: {}", sheet.headers.join(" | "));
        println!("Row template: {}", template.row_template);
        if !sheet.intro_lines.is_empty() {
            println!("Intro lines:");
            for line in sheet.intro_lines.iter().take(8) {
                println!("  {line}");
            }
        }

        if cli.template_only {
            continue;
        }

        let mapping = infer_field_mapping(&sheet.headers);
        println!("Field mapping source: heuristic");

        let default_currency = infer_currency_from_intro(&sheet.intro_lines);
        let txns = extract_transactions(&sheet, &mapping, default_currency.as_deref());

        println!("Extracted transactions: {}", txns.len());
        for (idx, t) in txns.iter().take(8).enumerate() {
            println!(
                "{:>3}. date={} amount={} currency={} bill_id={:?} ref={}",
                idx + 1,
                t.transaction_date_raw
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                t.amount,
                t.currency,
                t.bill_id,
                t.transaction_reference
                    .clone()
                    .unwrap_or_else(|| "-".to_string())
            );
        }
    }

    Ok(())
}
