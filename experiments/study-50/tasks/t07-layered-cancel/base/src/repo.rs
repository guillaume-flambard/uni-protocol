pub use crate::store::Store;

/// The only module allowed to touch `Store` internals.
pub fn get(store: &Store, id: u32) -> Option<&crate::domain::Booking> {
    store.rows.iter().find(|b| b.id == id)
}

pub fn get_mut(store: &mut Store, id: u32) -> Option<&mut crate::domain::Booking> {
    store.rows.iter_mut().find(|b| b.id == id)
}
