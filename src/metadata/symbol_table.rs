use crate::metadata::ast::{Ast, NodeKind, TypeDecType, Visitable};
use indextree::{Arena, NodeId};
use std::{error, fmt};
use tower_lsp::lsp_types::Range;

#[derive(Debug, Default)]
pub struct SymbolTable {
    arena: Arena<ScopeSymbolTable>,
    root_id: Option<NodeId>,
}

impl SymbolTable {
    pub fn new(ast: &Ast) -> SymbolTable {
        let mut table = SymbolTable::default();

        table.root_id = Some(table.parse_scope(ast.get_root_id(), ast));

        table
    }

    fn parse_scope(&mut self, scope_node_id: NodeId, ast: &Ast) -> NodeId {
        let table = ScopeSymbolTable {
            types: self.parse_type_decs(scope_node_id, ast),
        };
        debug!("{:?}", table);
        let node_id = self.arena.new_node(table);

        for subscope_id in ast.get_subscope_ids(scope_node_id) {
            let subtable = self.parse_scope(subscope_id, ast);
            node_id.append(subtable, &mut self.arena);
        }

        node_id
    }

    fn parse_type_decs(
        &self,
        scope_node_id: NodeId,
        ast: &Ast,
    ) -> Vec<Result<Symbol, SymbolError>> {
        let mut types: Vec<Result<Symbol, SymbolError>> = vec![];

        for node_id in ast.get_child_ids(scope_node_id) {
            let node = ast.get_node(node_id);

            if let NodeKind::TypeDec(type_dec_type) = &node.kind {
                let name_node_id = ast.get_child_of_kind(node_id, NodeKind::Name).unwrap();
                let name = ast.get_node(name_node_id).content.clone();

                let symbol = match type_dec_type {
                    TypeDecType::TypeDef => Ok(Symbol {
                        name,
                        def_position: node.range,
                    }),
                    _ => Err(SymbolError::Unknown),
                };

                types.push(symbol);
            }
        }

        types
    }
}

#[derive(Debug, Default)]
struct ScopeSymbolTable {
    types: Vec<Result<Symbol, SymbolError>>,
}

#[derive(Debug, Default)]
struct Symbol {
    name: String,
    def_position: Range,
}

#[derive(Debug)]
enum SymbolError {
    InvalidType,
    Unknown,
}

impl error::Error for SymbolError {}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match self {
            SymbolError::InvalidType => "Invalid type.",
            SymbolError::Unknown => "Unknown error.",
        };

        write!(f, "{}", message)
    }
}
