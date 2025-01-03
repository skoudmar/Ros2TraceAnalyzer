use bt2_derive::TryFromBtFieldConst;

#[derive(TryFromBtFieldConst)]
pub struct Event {
    data: u64,
}

fn main() {}
