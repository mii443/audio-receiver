use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub token: String,
    pub application_id: u64,
}
