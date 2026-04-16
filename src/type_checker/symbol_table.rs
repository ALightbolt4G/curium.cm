use crate::parser::ast::Type;
use std::collections::HashMap;

/// Describes a symbol in the Curium program.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: Type,
    pub mutable: bool,
    pub kind: SymbolKind,
}

impl Symbol {
    pub fn kind_name(&self) -> &'static str {
        match &self.kind {
            SymbolKind::Variable => "variable",
            SymbolKind::Function { .. } => "function",
            SymbolKind::Struct { .. } => "struct",
            SymbolKind::Enum { .. } => "enum",
            SymbolKind::Trait { .. } => "trait",
            SymbolKind::Method => "method",
            SymbolKind::Parameter => "parameter",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function {
        params: Vec<Type>,
        return_type: Type,
    },
    Struct {
        fields: Vec<(String, Type)>,
    },
    Enum {
        variants: Vec<(String, Vec<Type>)>,
    },
    Trait {
        methods: Vec<(String, Vec<Type>, Type)>,
    },
    Method,
    Parameter,
}

/// A scope in the symbol table.
#[derive(Debug, Clone)]
pub struct Scope {
    pub kind: ScopeKind,
    pub symbols: HashMap<String, Symbol>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScopeKind {
    Global,
    Function,
    Block,
    Loop,
    Reactor,
    Impl(String),
}

/// Hierarchical symbol table with lexical scoping.
#[derive(Debug)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self {
            scopes: Vec::new(),
        };
        table.push_scope(ScopeKind::Global);
        table.register_builtins();
        table
    }

    fn register_builtins(&mut self) {
        // print(s: string)
        self.define(Symbol {
            name: "print".to_string(),
            ty: Type::Void,
            mutable: false,
            kind: SymbolKind::Function {
                params: vec![Type::String],
                return_type: Type::Void,
            },
        });

        // println(s: string)
        self.define(Symbol {
            name: "println".to_string(),
            ty: Type::Void,
            mutable: false,
            kind: SymbolKind::Function {
                params: vec![Type::String],
                return_type: Type::Void,
            },
        });
    }

    pub fn push_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope {
            kind,
            symbols: HashMap::new(),
        });
    }

    pub fn pop_scope(&mut self) -> Option<Scope> {
        if self.scopes.len() > 1 {
            self.scopes.pop()
        } else {
            None
        }
    }

    pub fn define(&mut self, symbol: Symbol) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.symbols.insert(symbol.name.clone(), symbol);
        }
    }

    /// Look up a symbol by walking outward through scopes.
    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(sym) = scope.symbols.get(name) {
                return Some(sym);
            }
        }
        None
    }

    /// Check if we're currently inside a loop.
    pub fn in_loop(&self) -> bool {
        self.scopes.iter().rev().any(|s| s.kind == ScopeKind::Loop)
    }

    /// Check if we're inside a reactor block.
    pub fn in_reactor(&self) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|s| s.kind == ScopeKind::Reactor)
    }

    /// Get the current function's return type (if inside one).
    pub fn current_function_return_type(&self) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if scope.kind == ScopeKind::Function {
                // The function symbol is in the parent scope
                // We store the return type when we enter
                return None; // handled by checker via context
            }
        }
        None
    }

    pub fn current_scope_kind(&self) -> &ScopeKind {
        &self.scopes.last().unwrap().kind
    }

    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    /// Return all symbols in the global (bottom) scope.
    pub fn global_symbols(&self) -> Vec<&Symbol> {
        if let Some(scope) = self.scopes.first() {
            scope.symbols.values().collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_resolution() {
        let mut table = SymbolTable::new();

        table.define(Symbol {
            name: "x".to_string(),
            ty: Type::I32,
            mutable: false,
            kind: SymbolKind::Variable,
        });

        assert!(table.resolve("x").is_some());
        assert!(table.resolve("y").is_none());

        // Enter inner scope
        table.push_scope(ScopeKind::Block);
        table.define(Symbol {
            name: "y".to_string(),
            ty: Type::Bool,
            mutable: true,
            kind: SymbolKind::Variable,
        });

        // Both visible from inner scope
        assert!(table.resolve("x").is_some());
        assert!(table.resolve("y").is_some());

        // Exit inner scope
        table.pop_scope();
        assert!(table.resolve("x").is_some());
        assert!(table.resolve("y").is_none());
    }

    #[test]
    fn test_builtins_registered() {
        let table = SymbolTable::new();
        assert!(table.resolve("print").is_some());
        assert!(table.resolve("println").is_some());
    }

    #[test]
    fn test_loop_detection() {
        let mut table = SymbolTable::new();
        assert!(!table.in_loop());
        table.push_scope(ScopeKind::Loop);
        assert!(table.in_loop());
        table.pop_scope();
        assert!(!table.in_loop());
    }
}
