use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct ImageEntry {
    pub index: usize,
    pub path: String,
    pub thumbnail: String,
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
    Date,    // newest first
    DateOld, // oldest first
    Size,    // largest first
    SizeAsc, // smallest first
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PostCommand {
    cmds: Vec<String>,
}
