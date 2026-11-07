use super::*;

fn rejects_append(owner: ReadOnlyOwner) {
    let _result = owner.append(crate::audit::record::Record::default());
}
