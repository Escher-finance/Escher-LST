use vergen_git2::{Emitter, Git2Builder};

fn main() -> Result<(), anyhow::Error> {
    Emitter::default()
        .add_instructions(&Git2Builder::all_git()?)?
        .emit()
}
