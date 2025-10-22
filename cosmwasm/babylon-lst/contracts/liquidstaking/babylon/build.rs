use vergen_git2::{Emitter, Git2Builder};

fn main() {
    Emitter::default()
        .add_instructions(&Git2Builder::all_git().unwrap())
        .unwrap()
        .fail_on_error()
        .emit()
        .unwrap();
}
