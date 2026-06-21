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

    pub(crate) fn fn_lambda(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.is_empty() {
            return CalcResult::new_args_number_error(cell);
        }
        let mut params = Vec::new();
        for param in &args[..args.len() - 1] {
            match param {
                Node::WrongVariableKind(s) => params.push(s.to_uppercase()),
                _ => {
                    return CalcResult::new_error(
                        Error::VALUE,
                        cell,
                        "LAMBDA expects parameter names".to_string(),
                    );
                }
            }
        }
        CalcResult::Lambda {
            params,
            body: Box::new(args[args.len() - 1].clone()),
            captured: self.let_scopes.clone(),
        }
    }

    pub(crate) fn fn_isomitted(&mut self, args: &[Node], cell: CellReferenceIndex) -> CalcResult {
        if args.len() != 1 {
            return CalcResult::new_args_number_error(cell);
        }
        let key = match &args[0] {
            Node::WrongVariableKind(s) => s.to_uppercase(),
            _ => {
                let value = self.evaluate_node_in_context(&args[0], cell);
                return CalcResult::Boolean(matches!(value, CalcResult::EmptyArg));
            }
        };
        for scope in self.let_scopes.iter().rev() {
            if let Some(value) = scope.get(&key) {
                return CalcResult::Boolean(matches!(value, CalcResult::EmptyArg));
            }
        }
        CalcResult::Boolean(false)
    }
}
