#![expect(non_upper_case_globals)]
#![expect(non_camel_case_types)]
#![allow(unused)]
#![allow(clippy::use_self)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
