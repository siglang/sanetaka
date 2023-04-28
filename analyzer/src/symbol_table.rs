use parser::ast::DataType;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolEntry {
    pub data_type: DataType,
    pub attributes: SymbolAttributes,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymbolAttributes {
    pub function_return_type: Option<DataType>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SymbolTable {
    pub entries: HashMap<String, SymbolEntry>,
    pub parent: Option<Box<SymbolTable>>,
}

impl SymbolTable {
    pub fn new(parent: Option<SymbolTable>) -> Self {
        Self {
            entries: HashMap::new(),
            parent: parent.map(Box::new),
        }
    }

    pub fn insert(&mut self, name: &str, entry: SymbolEntry) {
        self.entries.insert(name.to_string(), entry);
    }

    pub fn lookup(&self, name: &str) -> Option<&SymbolEntry> {
        self.entries
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.lookup(name)))
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut SymbolEntry> {
        self.entries.get_mut(name).or_else(|| {
            self.parent
                .as_mut()
                .and_then(|parent| parent.lookup_mut(name))
        })
    }
}
