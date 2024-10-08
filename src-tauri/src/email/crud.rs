use super::{Email, EmailFilters};
use crate::error::DwataError;
use crate::workspace::crud::{
    CRUDCreateUpdate, CRUDRead, CRUDReadFilter, InputValue, VecColumnNameValue,
};

impl CRUDReadFilter for EmailFilters {
    fn get_column_names_values_to_filter(&self) -> VecColumnNameValue {
        let mut name_values: VecColumnNameValue = VecColumnNameValue::default();
        if let Some(x) = &self.email_account_id_list {
            name_values.push_name_value("email_account_id_list", InputValue::IDList(x.clone()));
        }
        if let Some(x) = &self.search_query {
            name_values.push_name_value("search_query", InputValue::Text(x.clone()));
        }
        name_values
    }
}

// impl CRUDCreateUpdate for Email {
//     fn table_name() -> String {
//         "email".to_string()
//     }

//     fn get_parsed_item(&self) -> Result<VecColumnNameValue, DwataError> {
//         let mut names_values: VecColumnNameValue = VecColumnNameValue::default();
//         if let Some(x) = &self.parent_email_id {
//             names_values.push_name_value("parent_email_id", InputValue::ID(*x));
//         }

//         Ok(names_values)
//     }
// }

impl CRUDRead for Email {
    fn table_name() -> String {
        "email".to_string()
    }
}
