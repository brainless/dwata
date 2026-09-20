//! Maps names as they literally appear in bank narrations to a resolved
//! `Party`.
//!
//! Real names are personally identifying and must never be committed to
//! this (public) repository, so this registry is always loaded from a
//! local, gitignored JSON file (see `usecases/*/local/` in `.gitignore`)
//! rather than being hardcoded in source. The crate's committed code only
//! knows the shape of the lookup, never real identities.

use crate::party::Party;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PartyRegistryEntry {
    /// Matched as a case-insensitive substring of a narration.
    pub name_fragment: String,
    pub party: Party,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PartyRegistry {
    entries: Vec<PartyRegistryEntry>,
}

impl PartyRegistry {
    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Resolve a narration to a known `Party`, if any registered name
    /// fragment appears in it. Longer fragments are checked first so a
    /// more specific match (e.g. a full name) wins over a shorter one
    /// (e.g. a shared surname).
    pub fn resolve(&self, narration: &str) -> Option<Party> {
        let narration_lower = narration.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| narration_lower.contains(&entry.name_fragment.to_lowercase()))
            .max_by_key(|entry| entry.name_fragment.len())
            .map(|entry| entry.party.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::party::PersonRelationship;

    #[test]
    fn resolves_known_name_fragment_case_insensitively() {
        let registry = PartyRegistry {
            entries: vec![PartyRegistryEntry {
                name_fragment: "Jane Doe".to_string(),
                party: Party::Person {
                    name: "Jane Doe".to_string(),
                    relationship: PersonRelationship::Spouse,
                },
            }],
        };

        let resolved = registry.resolve("NEFT DR-BANK0001-JANE DOE-NETBANK-SALARY JUNE");
        assert!(matches!(
            resolved,
            Some(Party::Person {
                relationship: PersonRelationship::Spouse,
                ..
            })
        ));
    }

    #[test]
    fn returns_none_for_unknown_name() {
        let registry = PartyRegistry::default();
        assert!(registry.resolve("SOME UNKNOWN NARRATION").is_none());
    }
}
