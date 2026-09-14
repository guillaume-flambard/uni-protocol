#[derive(Debug, Clone, PartialEq)]
pub struct Booking {
    pub id: u32,
    pub seats: u32,
    pub cancelled: bool,
}
