use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct Image {
    pub url: String,
    pub caption: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct Link {
    pub url: String,
    pub caption: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export, rename_all = "camelCase")]
pub struct Code {
    pub language: String,
    pub lines: Vec<(usize, String)>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Hash, TS)]
#[ts(export)]
pub enum TextType {
    Email,
    Password,
    SingleLine,
    MultiLine,
    Link,
    Code,
    FilePath,
}

#[derive(Deserialize, Serialize, TS)]
#[ts(export)]
pub enum ContentType {
    Text,
    ID,
    TextArray,
    SingleChoice,
    // Image,
    // DateTime,
}

#[derive(Default, Deserialize, Serialize, PartialEq, Eq, Hash, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ContentSpec {
    pub text_type: Option<TextType>,
    pub length_limits: Option<(usize, usize)>,
    pub choices: Option<Vec<(String, String)>>,
    // The frontend will call the provided function to get choices,
    // generally needed when choices come from a data source
    pub choices_from_function: Option<String>,
    // Text can be a prompt to AI model
    pub is_prompt: Option<bool>,
    // BulletPoints,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub enum Content {
    Text(String),
    ID(i64),
    // Image(Image),
    // DateTime(DateTime<Utc>),
}
