use alloc::string::{String, ToString};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    name: String,
    value: String,
}

impl Attribute {
    pub fn new() -> Self {
        Self {
            name: "".to_string(),
            value: "".to_string(),
        }
    }

    pub fn add_name(&mut self, c: char) {
        self.name.push(c);
    }

    pub fn add_value(&mut self, c: char) {
        self.value.push(c);
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn value(&self) -> String {
        self.value.clone()
    }
}
