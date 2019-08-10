
use quote::quote;

use proc_macro2::TokenStream;

use crate::logic::validation::register_description::RegisterDescription;

use super::ident;

pub fn register_trait_module(rd: &RegisterDescription) -> TokenStream {
    let index_type = ident(rd.index_size.rust_unsigned_integer());
    let address_type = rd.address_size.rust_type();
    quote! {
        pub mod register_trait {
            pub trait LocationIndexR {
                const INDEX_R: #index_type;
            }

            pub trait LocationAbsR {
                const ABS_ADDRESS_R: #address_type;
            }

            pub trait LocationRelR {
                const REL_ADDRESS_R: #address_type;
            }

            pub trait LocationIndexW {
                const INDEX_W: #index_type;
            }

            pub trait LocationAbsW {
                const ABS_ADDRESS_W: #address_type;
            }

            pub trait LocationRelW {
                const REL_ADDRESS_W: #address_type;
            }

            pub trait RegisterIndexIoR<T: RegisterGroup, U: Sized> {
                fn read(&mut self, index: #index_type) -> U;
            }

            pub trait RegisterIndexIoW<T: RegisterGroup, U: Sized> {
                fn write(&mut self, index: #index_type, value: U);
            }

            pub trait RegisterAbsIoR<T: RegisterGroup, U: Sized> {
                fn read(&mut self, abs_address: #address_type) -> U;
            }

            pub trait RegisterAbsIoW<T: RegisterGroup, U: Sized> {
                fn write(&mut self, abs_address: #address_type, value: U);
            }

            pub trait RegisterRelIoR<T: RegisterGroup, U: Sized> {
                fn read(&mut self, rel_address: #address_type) -> U;
            }

            pub trait RegisterRelIoW<T: RegisterGroup, U: Sized> {
                fn write(&mut self, rel_address: #address_type, value: U);
            }

            pub trait RegisterGroup {}

            pub trait InGroup {
                type Group: RegisterGroup;
            }
        }
    }
}
