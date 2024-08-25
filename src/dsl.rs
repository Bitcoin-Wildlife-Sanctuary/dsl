use crate::data_type::{DataTypeMetadata, DataTypeRegistry};
use crate::functions::{FunctionMetadata, FunctionRegistry};
use crate::treepp::pushable::{Builder, Pushable};
use indexmap::IndexMap;

pub struct DSL {
    pub data_type_registry: DataTypeRegistry,
    pub function_registry: FunctionRegistry,
    pub memory: IndexMap<usize, MemoryEntry>,
    pub memory_last_idx: usize,
    pub trace: Vec<TraceEntry>,
    pub num_inputs: Option<usize>,
    pub hint: Vec<MemoryEntry>,
}

#[derive(Clone, Debug)]
pub struct MemoryEntry {
    pub data_type: String,
    pub data: Element,
    pub description: Option<String>,
}

impl Pushable for &MemoryEntry {
    fn bitcoin_script_push(&self, builder: Builder) -> Builder {
        (&self.data).bitcoin_script_push(builder)
    }
}

#[derive(Clone, Debug)]
pub enum Element {
    Num(i32),
    ManyNum(Vec<i32>),
    Str(Vec<u8>),
    ManyStr(Vec<Vec<u8>>),
}

impl Pushable for &Element {
    fn bitcoin_script_push(&self, mut builder: Builder) -> Builder {
        match self {
            Element::Num(v) => v.bitcoin_script_push(builder),
            Element::ManyNum(v) => {
                for vv in v.iter() {
                    builder = vv.bitcoin_script_push(builder);
                }
                builder
            }
            Element::Str(v) => v.bitcoin_script_push(builder),
            Element::ManyStr(v) => {
                for vv in v.iter() {
                    builder = vv.bitcoin_script_push(builder);
                }
                builder
            }
        }
    }
}

#[derive(Clone)]
pub enum TraceEntry {
    FunctionCall(String, Vec<usize>),
    AllocatedConstant(usize),
}

impl Element {
    pub fn len(&self) -> usize {
        match self {
            Element::Num(_) => 1,
            Element::ManyNum(v) => v.len(),
            Element::Str(_) => 1,
            Element::ManyStr(v) => v.len(),
        }
    }
}

impl MemoryEntry {
    pub fn new(data_type: impl ToString, data: Element) -> Self {
        Self {
            data_type: data_type.to_string(),
            data,
            description: None,
        }
    }

    pub fn new_with_description(
        data_type: impl ToString,
        data: Element,
        description: impl ToString,
    ) -> Self {
        Self {
            data_type: data_type.to_string(),
            data,
            description: Some(description.to_string()),
        }
    }
}

impl DSL {
    pub fn new() -> Self {
        Self {
            data_type_registry: DataTypeRegistry::new(),
            function_registry: FunctionRegistry::new(),
            memory: IndexMap::new(),
            memory_last_idx: 0,
            trace: vec![],
            num_inputs: None,
            hint: vec![],
        }
    }

    pub fn add_data_type(&mut self, name: impl ToString, num_elements: usize) {
        self.data_type_registry.map.insert(
            name.to_string(),
            DataTypeMetadata {
                num_elements,
                ref_only: false,
            },
        );
    }

    pub fn add_ref_only_data_type(&mut self, name: impl ToString, num_elements: usize) {
        self.data_type_registry.map.insert(
            name.to_string(),
            DataTypeMetadata {
                num_elements,
                ref_only: true,
            },
        );
    }

    pub fn add_function(&mut self, name: impl ToString, meta: FunctionMetadata) {
        self.function_registry.map.insert(name.to_string(), meta);
    }

    fn alloc(&mut self, data_type: impl ToString, data: Element) -> usize {
        let idx = self.memory_last_idx;
        self.memory_last_idx += 1;
        assert!(self
            .data_type_registry
            .map
            .get(&data_type.to_string())
            .is_some());
        assert!(self.memory.get(&idx).is_none());
        self.memory.insert(
            idx,
            MemoryEntry {
                data_type: data_type.to_string(),
                data,
                description: None,
            },
        );
        idx
    }

    pub fn alloc_constant(&mut self, data_type: impl ToString, data: Element) -> usize {
        if self.num_inputs.is_none() {
            self.num_inputs = Some(self.memory_last_idx);
        }
        let idx = Self::alloc(self, data_type, data);
        self.trace.push(TraceEntry::AllocatedConstant(idx));
        idx
    }

    pub fn alloc_input(&mut self, data_type: impl ToString, data: Element) -> usize {
        assert!(self.num_inputs.is_none());
        Self::alloc(self, data_type, data)
    }

    pub fn get_num(&mut self, idx: usize) -> i32 {
        match self.memory.get(&idx).unwrap().data {
            Element::Num(v) => v,
            _ => {
                unimplemented!()
            }
        }
    }

    pub fn set_name(&mut self, idx: usize, name: impl ToString) {
        self.memory.get_mut(&idx).unwrap().description = Some(name.to_string());
    }

    pub fn execute(&mut self, function_name: impl ToString, input_idxs: &[usize]) -> Vec<usize> {
        if self.num_inputs.is_none() {
            self.num_inputs = Some(self.memory_last_idx);
        }

        assert!(self
            .function_registry
            .map
            .get(&function_name.to_string())
            .is_some());

        let function_metadata = self
            .function_registry
            .map
            .get(&function_name.to_string())
            .unwrap();

        assert_eq!(function_metadata.input.len(), input_idxs.len());

        for (input_idx, input_type) in input_idxs.iter().zip(function_metadata.input.iter()) {
            let stack_entry = self.memory.get_mut(input_idx).unwrap();
            assert_eq!(stack_entry.data_type, *input_type);
        }

        let output_types = function_metadata.output.clone();

        let exec_result = (function_metadata.trace_generator)(self, &input_idxs);

        assert_eq!(exec_result.new_elements.len(), output_types.len());

        self.hint.extend(exec_result.new_hints);

        let mut outputs = vec![];
        for (output_type, entry) in output_types.iter().zip(exec_result.new_elements) {
            assert_eq!(*output_type, entry.data_type);

            let idx = self.memory_last_idx;
            self.memory_last_idx += 1;
            self.memory.insert(idx, entry);
            outputs.push(idx);
        }

        self.trace.push(TraceEntry::FunctionCall(
            function_name.to_string(),
            input_idxs.to_vec(),
        ));

        outputs
    }
}
