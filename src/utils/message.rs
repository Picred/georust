#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Message {
    pub body: String
}
