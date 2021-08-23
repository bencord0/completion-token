#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub value: String,
}

impl Response {
    pub fn new() -> Self {
        let value = String::default();
        Self { value }
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}
