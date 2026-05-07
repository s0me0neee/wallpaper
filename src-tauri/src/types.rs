use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct ImageEntry {
    pub path: String,
    pub thumbnail: String,
}

#[derive(Serialize, Clone)]
pub struct LoadDone {
    pub loaded: usize,
    pub skipped: usize,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    Name,
    NameDesc,
    Date,    // newest first
    DateOld, // oldest first
    Size,    // largest first
    SizeAsc, // smallest first
}
