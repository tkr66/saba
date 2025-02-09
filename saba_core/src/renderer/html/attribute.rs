use alloc::string::String;

pub struct Attribute {
    name: String,
    value: String,
}

impl Attribute {
    pub fn new(name: String, value: String) -> Self {
        Self { name, value }
    }

    pub fn add_char(&mut self, ch: char, is_name: bool) {
        if is_name {
            self.name.push(ch);
        } else {
            self.value.push(ch);
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn value(&self) -> String {
        self.value.clone()
    }
}
