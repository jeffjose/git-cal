//! Tells the build which commit it is of, for `cal --version`: the short
//! SHA, and whether the tree had changes not committed. Built where there
//! is no git, as from a crate's package, vergen says so with a stand-in
//! that `version()` leaves out.

use vergen_gitcl::{Emitter, Gitcl};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Untracked files are not counted: a stray file is no change to cal.
    let git = Gitcl::builder().sha(true).dirty(false).build();
    Emitter::default().add_instructions(&git)?.emit()?;
    Ok(())
}
