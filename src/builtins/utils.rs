use crate::treepp::*;
use bitcoin::opcodes::all::OP_RETURN;

#[allow(unused)]
pub(crate) fn return_script() -> Script {
    Script::from(vec![OP_RETURN.to_u8()])
}
