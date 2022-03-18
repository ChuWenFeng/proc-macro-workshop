use derive_debug::CustomDebug;
use std::fmt::Debug;

pub trait Trait {
    type Value;
}

// #[derive(CustomDebug)]
// pub struct Fieldt {
//     name: &'static str,
//     #[debug = "0b{:08b}"]
//     bitmask: u8,
// }

#[derive(CustomDebug)]
pub struct Wrapper<T: Trait, U> {
    #[debug(bound = "T::Value: Debug")]
    field: Field<T>,
    normal: U,
}

#[derive(CustomDebug)]
struct Field<T: Trait> {
    values: Vec<T::Value>,
}

fn assert_debug<F: Debug>() {}

fn main() {
    struct Id;

    impl Trait for Id {
        type Value = u8;
    }

    assert_debug::<Wrapper<Id,u8>>();
}
