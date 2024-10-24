#[macro_export]
macro_rules! impl_deref {
    ($into:ty; for $($typ:ty),+) => {
        $(
            impl Deref for $typ {
                type Target = $into;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }
        )+
    };
}
