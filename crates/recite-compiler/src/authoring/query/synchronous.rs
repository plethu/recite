use crate::authoring::CancellationToken;
use crate::authoring::{
    AuthoringSnapshot, NavigationResult, QueryResult, SymbolLocation, SymbolQueryOptions,
};
use recite_core::{DocumentKey, SourcePosition};
impl AuthoringSnapshot {
    pub fn symbols(
        &self,
        key: &DocumentKey,
        options: SymbolQueryOptions,
    ) -> QueryResult<Vec<SymbolLocation>> {
        self.query(&CancellationToken::new()).symbols(key, options)
    }
    pub fn project_symbols(&self, options: SymbolQueryOptions) -> QueryResult<Vec<SymbolLocation>> {
        self.query(&CancellationToken::new())
            .project_symbols(options)
    }
    pub fn references(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
        options: SymbolQueryOptions,
    ) -> QueryResult<Vec<SymbolLocation>> {
        self.query(&CancellationToken::new())
            .references(key, position, options)
    }
    pub fn navigate(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
    ) -> QueryResult<NavigationResult> {
        self.query(&CancellationToken::new())
            .navigate(key, position)
    }
}
