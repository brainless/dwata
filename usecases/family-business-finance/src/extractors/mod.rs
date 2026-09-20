//! Source-specific extractors. Each submodule targets one concrete source
//! format the owner actually has (see `log/`) and must produce a
//! `Transaction` for every data row — ambiguity becomes
//! `TransactionKind::Unresolved`, never a dropped row or a parse failure.

pub mod hdfc_bank_csv;
