use crate::domain::Booking;

/// Raw storage. Only `repo` may touch this module's fields directly; the
/// service layer goes through `repo` instead (declared architecture rule).
pub struct Store {
    pub rows: Vec<Booking>,
}
