use std::collections::HashMap;
use super::types::PrismType;

/// A symbol in the symbol table.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: PrismType,
    pub kind: SymbolKind,
    pub mutable: bool,
    pub definition_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    /// Top-level type (component, asset, class, enum)
    Type,
    /// Field on a component/asset/class
    Field,
    /// Serialized field
    SerializeField,
    /// require/child/parent — non-null, auto-resolved
    RequiredComponent,
    /// optional — nullable, auto-resolved
    OptionalComponent,
    /// Function
    Function,
    /// Coroutine
    Coroutine,
    /// Function parameter
    Parameter,
    /// Local variable (val/var)
    Local,
}

/// Scoped symbol table — stack of scopes.
#[derive(Debug)]
pub struct ScopeStack {
    scopes: Vec<HashMap<String, Symbol>>,
}

impl ScopeStack {
    pub fn new() -> Self {
        ScopeStack {
            scopes: vec![HashMap::new()], // global scope
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn define(&mut self, symbol: Symbol) -> bool {
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(&symbol.name) {
            return false; // duplicate
        }
        scope.insert(symbol.name.clone(), symbol);
        true
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.get(name) {
                return Some(sym);
            }
        }
        None
    }

    pub fn lookup_current_scope(&self, name: &str) -> Option<&Symbol> {
        self.scopes.last().and_then(|s| s.get(name))
    }
}
