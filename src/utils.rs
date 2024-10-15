#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Known<T> {
    Known(T),
    #[default]
    Unknown,
}
