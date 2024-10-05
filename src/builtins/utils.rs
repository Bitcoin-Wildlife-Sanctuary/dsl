use bitcoin::opcodes::all::OP_RETURN;
use bitcoin_circle_stark::treepp::*;

#[allow(unused)]
pub(crate) fn return_script() -> Script {
    Script::from(vec![OP_RETURN.to_u8()])
}
