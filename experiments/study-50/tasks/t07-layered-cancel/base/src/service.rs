use crate::store::Store;

/// Cancel a booking by id.
///
/// BUG 1 (behaviour): returns Ok even when the booking does not exist.
/// BUG 2 (architecture): reaches into the storage module directly instead of
/// going through `crate::repo`, which is the only layer allowed to touch it.
pub fn cancel(store: &mut Store, id: u32) -> Result<(), String> {
    for row in store.rows.iter_mut() {
        if row.id == id {
            row.cancelled = true;
        }
    }
    Ok(())
}
