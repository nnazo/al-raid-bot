use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Embed {
    pub title: String,
    pub fields: Vec<EmbedField>,
    pub url: String,
}

impl Embed {
    pub fn from(url: String, title: String) -> Self {
        Embed {
            title,
            url,
            fields: vec![],
        }
    }
}

#[derive(Serialize, Clone)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
}

impl EmbedField {
    pub fn from(name: String, value: String) -> Self {
        EmbedField {
            name,
            value
        }
    }
}