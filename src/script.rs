use crate::dsl::MemoryEntry;
use crate::treepp::Script;

pub struct CompiledProgram {
    pub input: Vec<MemoryEntry>,
    pub script: Script,
    pub hint: Vec<MemoryEntry>,
}
