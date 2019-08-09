
use quote::quote;

use proc_macro2::TokenStream;


pub fn register_trait_module() -> TokenStream {
    quote! {
        pub mod register_trait {
            pub trait LocationIndex {
                const INDEX: u8;
            }

            pub trait LocationAbs {
                const ABS_ADDRESS: usize;
            }

            pub trait LocationRel {
                const REL_ADDRESS: usize;
            }

            pub trait RegisterIndexIo<T: RegisterGroup, U: Sized> {
                fn read(&mut self, index: u8) -> U;
                fn write(&mut self, index: u8, value: U);
            }

            pub trait RegisterAbsIo<T: RegisterGroup, U: Sized> {
                fn read(&mut self, abs_address: usize) -> U;
                fn write(&mut self, abs_address: usize, value: U);
            }

            pub trait RegisterRelIo<T: RegisterGroup, U: Sized> {
                fn read(&mut self, rel_address: usize) -> U;
                fn write(&mut self, rel_address: usize, value: U);
            }

            pub trait RegisterGroup {}

            pub trait InGroup {
                type Group: RegisterGroup;
            }
        }
    }
}
