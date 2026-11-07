use super::*;

fn rejects_checkpoint(owner: ReadOnlyOwner) {
    let _result = owner.checkpoint();
}
