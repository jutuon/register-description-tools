pub mod register;
pub mod register_trait;


use std::{
    io::Write,
    process::Command,
    convert::TryInto,
    fmt,
    fs,
};

use quote::quote;
use syn::{Ident, LitInt, IntSuffix};
use proc_macro2::{Span, TokenStream};
use inflections::Inflect;

use crate::logic::{
    validation::{
        ParsedFile,
        Registers,
        register::{
            RegisterSize,
        },
    },
};

pub fn parsed_file_to_rust(parsed_file: &ParsedFile, output: &str) {
    let token_stream = convert_parsed_file_to_token_stream(parsed_file);

    let mut f = fs::File::create(output).unwrap();
    f.write_all(token_stream.to_string().as_bytes()).unwrap();
    drop(f);

    let rustfmt_result = Command::new("rustfmt")
        .arg(output)
        .status();

    match rustfmt_result {
        Err(e) => println!("running rustfmt failed: {}", e),
        Ok(status) => {
            if !status.success() {
                println!("running rustfmt failed, exit code: {:?}", status.code())
            }
        }
    }

}

fn convert_parsed_file_to_token_stream(parsed_file: &ParsedFile) -> TokenStream {
    let trait_module = register_trait::register_trait_module();


    let groups: Vec<TokenStream> = match &parsed_file.registers {
        None => vec![],
        Some(Registers::Groups(groups)) => {
            groups.iter().map(|(name, registers)| {
                let module_name = ident(name.to_snake_case());
                let group_str = name.to_pascal_case();
                let group_type = ident(format!("{}Group", group_str));
                let registers_modules = register::registers_to_module(&registers, &group_type);
                let register_group = register::register_group(&registers, &group_type, &group_str);
                quote! {
                    pub mod #module_name {
                        use super::register_trait::*;
                        #register_group
                        #registers_modules
                    }
                }
            }).collect()
        }
        Some(Registers::OnlyRegisters(registers)) => {
            let group_type = ident("RegisterGroup");
            let registers_modules = register::registers_to_module(&registers, &group_type);
            let register_group = register::register_group(&registers, &group_type, "");
            vec![
                quote! {
                    pub mod register {
                        use super::register_trait::*;
                        #register_group
                        #registers_modules
                    }
                }
            ]
        }
    };

    let additional_doc = parsed_file.description.description.as_ref().map(|description| {
        quote! {
            #![doc = ""]
            #![doc = #description]
        }
    }).unwrap_or_default();

    let doc = format!("Generated from register description `{}`", parsed_file.description.name.as_str());

    quote! {
        #![allow(non_camel_case_types)]
        #![doc = #doc]
        #additional_doc

        #trait_module

        #( #groups )*
    }
}

pub fn ident<T: AsRef<str>>(text: T) -> Ident {
    Ident::new(text.as_ref(), Span::call_site())
}

pub fn lit_int<T: TryInto<u64, Error=U>, U: fmt::Debug>(number: T) -> LitInt {
    LitInt::new(number.try_into().unwrap(), IntSuffix::None, Span::call_site())
}

impl RegisterSize {
    pub fn rust_unsigned_integer(&self) -> &str {
        let number_type = match self {
            RegisterSize::Size8 => "u8",
            RegisterSize::Size16 => "u16",
            RegisterSize::Size32 => "u32",
            RegisterSize::Size64 => "u64",
        };

        number_type
    }
}
