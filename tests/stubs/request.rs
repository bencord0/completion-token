#[derive(Debug, Clone)]
pub struct Request {
    pub value: String,
}

impl Request {
    pub fn new(value: &str) -> Self {
        let value = value.to_string();
        Self { value }
    }
}
