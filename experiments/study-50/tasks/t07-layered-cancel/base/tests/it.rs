use mini_app::domain::Booking;
use mini_app::store::Store;

fn store_with(seats: u32, cancelled: bool) -> Store {
    Store {
        rows: vec![Booking {
            id: 1,
            seats,
            cancelled,
        }],
    }
}

#[test]
fn store_holds_rows() {
    let store = store_with(2, false);
    assert_eq!(store.rows.len(), 1);
}
