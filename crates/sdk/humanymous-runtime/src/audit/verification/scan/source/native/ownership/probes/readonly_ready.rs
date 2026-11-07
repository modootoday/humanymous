use super::*;

fn rejects_ready(owner: ReadOnlyOwner) {
    let _result = owner.ready();
}
