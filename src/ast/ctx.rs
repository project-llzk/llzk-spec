use internment::Arena;
use num_bigint::BigUint;

use crate::ast::{Literal, Symbol};

/// Context that manages the lifetime of the AST.
pub struct AstContext {
    symbols_arena: Arena<str>,
    literals_arena: Arena<BigUint>,
}

impl AstContext {
    /// Creates a new context.
    pub fn new() -> Self {
        Self {
            symbols_arena: Arena::new(),
            literals_arena: Arena::new(),
        }
    }

    /// Returns a symbol allocated inside the context.
    pub fn symbol<'a>(&'a self, symbol: &str) -> Symbol<'a> {
        Symbol {
            inner: self.symbols_arena.intern(symbol),
        }
    }

    /// Creates a symbol of the given string.
    ///
    /// If the symbol is fresh the string is moved directly.
    pub fn new_symbol(&self, symbol: String) -> Symbol<'_> {
        Symbol {
            inner: self.symbols_arena.intern_string(symbol),
        }
    }

    /// Returns a literal allocated inside the context.
    pub fn literal<'a>(&'a self, literal: BigUint) -> Literal<'a> {
        Literal {
            inner: self.literals_arena.intern(literal),
        }
    }
}

impl Default for AstContext {
    fn default() -> Self {
        Self::new()
    }
}
