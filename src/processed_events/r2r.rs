use crate::impl_from_for_enum;

impl_from_for_enum! {
#[derive(Debug, Clone)]
pub enum Event {

}
}

impl std::fmt::Display for Event {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}