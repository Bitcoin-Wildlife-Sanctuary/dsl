use std::collections::HashMap;

pub struct DataTypeRegistry {
    pub map: HashMap<String, DataTypeMetadata>,
}

impl DataTypeRegistry {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
}

pub struct DataTypeMetadata {
    pub num_elements: usize,
    pub ref_only: bool,
}
