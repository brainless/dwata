use super::EmailFilters;
use crate::workspace::crud::{CRUDReadFilter, InputValue, VecColumnNameValue};

impl CRUDReadFilter for EmailFilters {
    fn get_column_names_values_to_filter(&self) -> VecColumnNameValue {
        let mut name_values: VecColumnNameValue = VecColumnNameValue::default();
        if let Some(x) = &self.mailbox_id {
            name_values.push_name_value("mailbox_id", InputValue::ID(*x));
        }
        if let Some(x) = &self.search_query {
            name_values.push_name_value("search_query", InputValue::Text(x.clone()));
        }
        name_values
    }
}
