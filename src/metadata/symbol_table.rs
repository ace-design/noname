use std::fmt;

use crate::metadata::ast::{Ast, NodeKind, VisitNode, Visitable};
use crate::metadata::types::Type;
use crate::utils;
use indextree::{Arena, NodeId};
use tower_lsp::lsp_types::{Position, Range};

#[derive(Debug, Default)]
pub struct SymbolTable {
    arena: Arena<ScopeSymbolTable>,
    root_id: Option<NodeId>,
}

pub trait SymbolTableActions {
    fn get_symbols_in_scope(&self, position: Position) -> Option<Symbols>;
    fn get_top_level_symbols(&self) -> Option<Symbols>;
    fn get_symbol_at_pos(&self, symbol: String, position: Position) -> Option<Symbol>;
}

impl SymbolTableActions for SymbolTable {
    fn get_symbols_in_scope(&self, position: Position) -> Option<Symbols> {
        let mut current_scope_id = self.root_id?;
        let mut symbols = self.arena.get(current_scope_id)?.get().symbols.clone();

        let mut subscope_exists = true;
        while subscope_exists {
            subscope_exists = false;

            for child_id in current_scope_id.children(&self.arena) {
                let scope = self.arena.get(child_id)?.get();
                if scope.range.start < position && position < scope.range.end {
                    current_scope_id = child_id;
                    subscope_exists = true;
                    symbols.add(scope.symbols.clone(), position);
                    break;
                }
            }
        }

        Some(symbols)
    }

    fn get_top_level_symbols(&self) -> Option<Symbols> {
        Some(self.arena.get(self.root_id?)?.get().symbols.clone())
    }

    fn get_symbol_at_pos(&self, symbol: String, position: Position) -> Option<Symbol> {
        todo!()
    }
}

impl SymbolTable {
    pub fn new(ast: &Ast) -> SymbolTable {
        let mut table = SymbolTable::default();

        table.root_id = Some(table.parse_scope(ast.visit_root(), ast));

        table
    }

    fn parse_scope(&mut self, visit_node: VisitNode, ast: &Ast) -> NodeId {
        let table = ScopeSymbolTable::parse(visit_node.clone());

        debug!("{}", table);
        let node_id = self.arena.new_node(table);

        for subscope_visit in visit_node.get_subscopes() {
            let subtable = self.parse_scope(subscope_visit, ast);
            node_id.append(subtable, &mut self.arena);
        }

        node_id
    }
}

#[derive(Debug, Default, Clone)]
pub struct Symbols {
    pub types: Vec<Symbol>,
    pub constants: Vec<Symbol>,
    pub variables: Vec<Symbol>,
    pub functions: Vec<Symbol>,
}

impl Symbols {
    fn position_filter(&mut self, position: Position) {
        self.types.retain(|s| s.def_position.end < position);
        self.constants.retain(|s| s.def_position.end < position);
        self.variables.retain(|s| s.def_position.end < position);
        self.functions.retain(|s| s.def_position.end < position);
    }

    pub fn add(&mut self, mut other: Symbols, position: Position) {
        other.position_filter(position);

        self.types.append(&mut other.types);
        self.constants.append(&mut other.constants);
        self.variables.append(&mut other.variables);
        self.functions.append(&mut other.functions);
    }
}

#[derive(Debug, Default)]
struct ScopeSymbolTable {
    range: Range,
    symbols: Symbols,
}

impl fmt::Display for ScopeSymbolTable {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let mut output = String::from("\n");

        output.push_str(
            format!(
                "{0: <8} | {1: <15} | {2: <10} | {3: <10} | {4: <10}\n",
                "symbol", "name", "position", "type", "usages"
            )
            .as_str(),
        );

        output.push_str("-".repeat(62).as_str());
        output.push('\n');

        for s in &self.symbols.types {
            output.push_str(format!("{: <8} | {}\n", "type", s.to_string()).as_str());
        }

        for s in &self.symbols.constants {
            output.push_str(format!("{: <8} | {}\n", "constant", s.to_string()).as_str());
        }

        for s in &self.symbols.variables {
            output.push_str(format!("{: <8} | {}\n", "variable", s.to_string()).as_str());
        }

        for s in &self.symbols.functions {
            output.push_str(format!("{: <8} | {}\n", "function", s.to_string()).as_str());
        }

        fmt.write_str(&output)
    }
}

impl ScopeSymbolTable {
    fn parse(root_visit_node: VisitNode) -> ScopeSymbolTable {
        let mut table = ScopeSymbolTable {
            range: root_visit_node.get().range,
            ..Default::default()
        };

        for child_visit_node in root_visit_node.get_children() {
            let child_node = child_visit_node.get();

            match &child_node.kind {
                NodeKind::ConstantDec => {
                    let name_node = child_visit_node.get_child_of_kind(NodeKind::Name).unwrap();
                    let name = name_node.get().content.clone();

                    let type_ = child_visit_node.get_type();

                    let symbol = Symbol::new(name, child_node.range, type_);

                    table.symbols.constants.push(symbol);
                }
                NodeKind::VariableDec => {
                    let name_node = child_visit_node.get_child_of_kind(NodeKind::Name).unwrap();
                    let name = name_node.get().content.clone();

                    let type_ = child_visit_node.get_type();

                    let symbol = Symbol::new(name, child_node.range, type_);

                    table.symbols.variables.push(symbol);
                }
                NodeKind::TypeDec(_type_dec_type) => {
                    let name_node = child_visit_node.get_child_of_kind(NodeKind::Name).unwrap();
                    let name = name_node.get().content.clone();

                    let type_ = child_visit_node.get_type();

                    table
                        .symbols
                        .types
                        .push(Symbol::new(name, child_node.range, type_));
                }
                _ => {}
            }
        }

        table
    }
}

#[derive(Debug, Clone)]
pub struct Symbol {
    name: String,
    def_position: Range,
    type_: Option<Type>,
    usages: Vec<Range>,
}

impl fmt::Display for Symbol {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(
            format!(
                "{0: <15} | {1: <10} | {2: <10} | {3: <10}",
                self.name,
                "",
                "",
                self.usages.len()
            )
            .as_str(),
        )
    }
}

impl Symbol {
    pub fn new(name: String, def_position: Range, type_: Option<Type>) -> Symbol {
        Symbol {
            name,
            def_position,
            type_,
            usages: vec![],
        }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}
