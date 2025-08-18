//! Repository indexing (Tree-sitter) - optional feature `repo-index`.
//! Provides API to index a source tree and emit JSONL entity records.
//!
//! High-level usage:
//! ```ignore
//! use goose::repo_index::{index_repository, RepoIndexOptions};
//! # use std::path::Path;
//! let opts = RepoIndexOptions::builder().root(Path::new(".")).build();
//! let stats = index_repository(opts).expect("index ok");
//! println!("indexed {} files", stats.files_indexed);
//! ```
#[cfg(feature = "repo-index")]
pub mod internal;
#[cfg(feature = "repo-index")]
pub mod service;
#[cfg(feature = "repo-index")]
pub mod tool;
#[cfg(feature = "repo-index")]
pub use internal::*;

#[cfg(not(feature = "repo-index"))]
pub mod disabled {
    use anyhow::Result;
    use std::path::Path;
    use std::time::Duration;
    #[derive(Debug, Clone, Default)]
    pub struct RepoIndexStats {
        pub files_indexed: usize,
        pub entities_indexed: usize,
        pub duration: Duration,
    }
    #[derive(Default)]
    pub struct RepoIndexOptions<'a> { pub _phantom: std::marker::PhantomData<&'a ()> }
    pub fn index_repository(_opts: RepoIndexOptions<'_>) -> Result<RepoIndexStats> {
        Err(anyhow::anyhow!("goose compiled without repo-index feature"))
    }
    impl<'a> RepoIndexOptions<'a> { pub fn builder() -> RepoIndexOptionsBuilder<'a>{ RepoIndexOptionsBuilder::default() } }
    #[derive(Default)]
    pub struct RepoIndexOptionsBuilder<'a>{ pub _phantom: std::marker::PhantomData<&'a ()> }
    impl<'a> RepoIndexOptionsBuilder<'a>{ pub fn root(self, _p: &'a Path)->Self{self} pub fn build(self)->RepoIndexOptions<'a>{ RepoIndexOptions{ _phantom: std::marker::PhantomData } } }
}
#[cfg(not(feature = "repo-index"))]
pub use disabled::*;
