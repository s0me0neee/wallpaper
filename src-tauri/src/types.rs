use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct ImageEntry {
    pub index: usize,
    pub path: String,
    pub thumbnail: String,
    pub modified: u64,
    pub size: u64,
}

#[derive(Serialize, Clone)]
pub struct LoadDone {
    pub loaded: usize,
    pub skipped: usize,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    Name,
    NameDesc,
    Date,
    DateOld,
    Size,
    SizeAsc,
}
