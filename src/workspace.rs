use std::collections::HashMap;

use petgraph::{prelude::NodeIndex, Graph};
use serde_json::Value;
use tower_lsp::lsp_types::{
    CompletionContext, CompletionItem, Diagnostic, HoverContents, Location, Position,
    SemanticTokensResult, TextDocumentContentChangeEvent, Url, WorkspaceEdit,
};

use crate::{file::File, settings::Settings};

pub trait FileManagement {
    fn get_file(&self, url: &Url) -> Option<&File>;
    fn get_file_mut(&mut self, url: &Url) -> Option<&mut File>;
    fn add_file(&mut self, url: Url, content: &str);
    fn update_file(&mut self, url: &Url, changes: Vec<TextDocumentContentChangeEvent>);
}

pub trait LanguageActions {
    fn get_definition_location(&self, url: &Url, symbol_position: Position) -> Option<Location>;
    fn get_semantic_tokens(&self, url: &Url) -> Option<SemanticTokensResult>;
    fn rename_symbol(
        &mut self,
        url: &Url,
        symbol_position: Position,
        new_name: String,
    ) -> Option<WorkspaceEdit>;
    fn get_completion(
        &self,
        url: &Url,
        position: Position,
        context: Option<CompletionContext>,
    ) -> Option<Vec<CompletionItem>>;
    fn get_hover_info(&self, url: &Url, position: Position) -> Option<HoverContents>;
    fn get_quick_diagnostics(&self, url: &Url) -> Vec<Diagnostic>;
    fn get_full_diagnostics(&self, url: &Url) -> Vec<Diagnostic>;
}

pub struct Workspace {
    settings: Settings,
    url_node_map: HashMap<Url, NodeIndex>,
    files_graph: Graph<File, ()>,
    tree_sitter_language: tree_sitter::Language,
}

impl Workspace {
    pub fn new(tree_sitter_language: tree_sitter::Language) -> Workspace {
        Workspace {
            settings: Settings::default(),
            url_node_map: HashMap::new(),
            files_graph: Graph::new(),
            tree_sitter_language,
        }
    }

    pub fn update_settings(&mut self, settings: Value) {
        self.settings = Settings::parse(settings);
        info!("Settings: {:?}", self.settings);
    }
}

impl FileManagement for Workspace {
    fn get_file(&self, url: &Url) -> Option<&File> {
        let index = self.url_node_map.get(url)?;
        self.files_graph.node_weight(*index)
    }

    fn get_file_mut(&mut self, url: &Url) -> Option<&mut File> {
        let index = self.url_node_map.get(url)?;
        self.files_graph.node_weight_mut(*index)
    }

    fn add_file(&mut self, url: Url, content: &str) {
        let index =
            self.files_graph
                .add_node(File::new(url.clone(), content, self.tree_sitter_language));
        self.url_node_map.insert(url, index);
    }

    fn update_file(&mut self, url: &Url, changes: Vec<TextDocumentContentChangeEvent>) {
        let file = self.get_file_mut(url).unwrap();

        file.update(changes);
    }
}

impl LanguageActions for Workspace {
    fn get_definition_location(&self, url: &Url, symbol_position: Position) -> Option<Location> {
        let file = self.get_file(url)?;

        file.get_definition_location(symbol_position)
    }

    fn rename_symbol(
        &mut self,
        url: &Url,
        symbol_position: Position,
        new_name: String,
    ) -> Option<WorkspaceEdit> {
        let file = self.get_file_mut(url).unwrap();

        file.rename_symbol(symbol_position, new_name)
    }

    fn get_semantic_tokens(&self, url: &Url) -> Option<SemanticTokensResult> {
        let file = self.get_file(url)?;

        Some(file.get_semantic_tokens())
    }

    fn get_completion(
        &self,
        url: &Url,
        position: Position,
        context: Option<CompletionContext>,
    ) -> Option<Vec<CompletionItem>> {
        let file = self.get_file(url)?;

        file.get_completion_list(position, context)
    }

    fn get_hover_info(&self, url: &Url, position: Position) -> Option<HoverContents> {
        let file = self.get_file(url)?;

        file.get_hover_info(position)
    }

    fn get_quick_diagnostics(&self, url: &Url) -> Vec<Diagnostic> {
        let maybe_file = self.get_file(url);

        if let Some(file) = maybe_file {
            file.get_quick_diagnostics()
        } else {
            vec![]
        }
    }

    fn get_full_diagnostics(&self, url: &Url) -> Vec<Diagnostic> {
        let maybe_file = self.get_file(url);

        if let Some(file) = maybe_file {
            file.get_full_diagnostics()
        } else {
            vec![]
        }
    }
}
