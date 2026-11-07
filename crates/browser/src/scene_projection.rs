//! Commit shell changes through Meari's retained source owner, including AX metadata.
use meari::domain::{Attribute, NodeKind, Value};
use meari_native::infrastructure::document::InteractiveDocument;

pub(crate) fn commit(doc: &mut InteractiveDocument) -> Result<(), String> {
    let mut source = doc.source.clone();
    source.states = crate::ui::native_states();
    if let NodeKind::Element { attributes, .. } = &mut source.root.kind {
        attributes.retain(|attribute| attribute.name != "data-browser-ready");
        attributes.push(Attribute {
            name: "data-browser-ready".into(),
            value: Value::State("browserReady".into()),
        });
    }
    let prepared = doc.prepare_source_replacement(source)?;
    doc.commit_source_replacement(prepared)
}
