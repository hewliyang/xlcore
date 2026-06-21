use std::collections::HashMap;

use crate::{
    calc_result::CalcResult,
    expressions::{parser::Node, token::Error, types::CellReferenceIndex},
    model::Model,
};

impl<'a> Model<'a> {
    pub(crate) fn fn_let(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() < 3 || args.len() % 2 == 0 {
            return CalcResult::new_args_number_error(cell);
        }
        self.let_scopes.push(HashMap::new());
        let pair_count = (args.len() - 1) / 2;
        for i in 0..pair_count {
            let name = match &args[2 * i] {
                Node::WrongVariableKind(s) => s.to_uppercase(),
                _ => {
                    self.let_scopes.pop();
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "LET expects a name".to_string(),
                    );
                }
            };
            let value = self.evaluate_node_in_context(&args[2 * i + 1], cell);
            if let Some(top) = self.let_scopes.last_mut() {
                top.insert(name, value);
            }
        }
        let result = self.evaluate_node_in_context(&args[args.len() - 1], cell);
        self.let_scopes.pop();
        result
    }
}
