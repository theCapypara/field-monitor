use gtk::prelude::*;

/// Clears the text of an Editable if it's `editable` property changes to `false`.
pub fn clear_editable_if_becoming_not_editable(w: &impl IsA<gtk::Editable>) {
    let w = w.upcast_ref();
    w.connect_notify(Some("editable"), move |w, _| {
        if !w.is_editable() {
            w.set_text("")
        }
    });
}
