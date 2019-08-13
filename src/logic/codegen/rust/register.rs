use std::{
    iter,
    collections::HashSet,
};

use quote::quote;
use syn::{Ident, LitInt};
use proc_macro2::{TokenStream};
use inflections::Inflect;

use crate::logic::{
    validation::{
        register::{
            RegisterLocation,
            Register,
            AccessMode,
            RegisterFunction,
            RegisterEnum,
            RegisterEnumValue,
            FunctionStatus,
        },
        register_description::{
            RegisterDescription,
        },
    },
};

use super::{ident, lit_int};


pub fn register_group(registers: &Vec<Register>, group_type: &Ident, group_name: &str) -> TokenStream {

    let mut unique_register_traits: HashSet<String> = HashSet::new();
    let mut register_traits: Vec<TokenStream> = vec![];

    for r in registers {
        for io_trait in r.io_traits_rust(group_type) {
            if unique_register_traits.insert(io_trait.to_string()) {
                register_traits.push(io_trait);
            }
        }
    }

    let type_bounds = quote! { #( #register_traits )+* };

    let register_getters_type = ident(format!("{}Registers", &group_name));

    let register_types_rust: Vec<Ident> = registers.iter().map(|r| r.register_rust_name()).collect();
    let register_types_rust_copy = register_types_rust.clone();
    let register_getters_rust: Vec<Ident> = registers.iter().map(|r| r.register_getter_rust_name()).collect();
    let docs: Vec<TokenStream> = registers.iter().map(|r| r.description_rust()).collect();

    let register_getters_read_access_rust: Vec<Ident> = registers.iter()
        .filter(|r| {
            r.access_mode == AccessMode::Read || r.access_mode == AccessMode::ReadWrite
        })
        .map(|r| r.register_getter_rust_name())
        .collect();

    quote! {
        pub struct #register_getters_type<T: #type_bounds > {
            io: T,
        }

        impl <T: #type_bounds > #register_getters_type<T> {
            #[inline]
            pub fn new(io: T) -> Self {
                Self {
                    io
                }
            }

            #(
                #docs
                #[inline]
                pub fn #register_getters_rust(&mut self) -> #register_types_rust<'_, T> {
                    #register_types_rust_copy {
                        io: &mut self.io
                    }
                }
            )*

            pub fn debug_registers<F: FnMut(&dyn core::fmt::Debug)>(&mut self, mut f: F) {
                #(
                    (f)(&self.#register_getters_read_access_rust().read());
                )*
            }
        }

        pub struct #group_type;
        impl RegisterGroup for #group_type {}

    }
}

pub fn registers_to_module(registers: &Vec<Register>, rd: &RegisterDescription, group_type: &Ident) -> TokenStream {

    let mut register_modules: Vec<TokenStream> = vec![];
    for r in registers {
        let module_name = ident(r.name.as_str().to_snake_case());
        let module = register_module(r,);
        let r_struct = register_struct(r, group_type);
        let r_struct_impl = register_struct_impl(r, rd, group_type);
        let tokens = quote! {
            #r_struct
            pub mod #module_name {
                #r_struct_impl
                #module
            }
        };

        register_modules.push(tokens);
    }


    quote! {
        #( #register_modules )*
    }
}


impl Register {
    fn description_rust(&self) -> TokenStream {
        self.description.as_ref().map(|description| quote!{ #[doc = #description] }).unwrap_or_default()
    }

    fn register_rust_name(&self) -> Ident {
        ident(self.name.as_str().to_constant_case())
    }

    fn register_getter_rust_name(&self) -> Ident {
        ident(self.name.as_str().to_snake_case())
    }

    fn io_traits_rust(&self, group_type: &Ident) -> Vec<TokenStream> {
        let size = ident(self.size_in_bits.rust_unsigned_integer());

        let mut r = vec![];

        if let AccessMode::Read | AccessMode::ReadWrite = self.access_mode {
            let read = match &self.read_location {
                RegisterLocation::Index(_) => quote! { RegisterIndexIoR<#group_type, #size> },
                RegisterLocation::Absolute(_) => quote! { RegisterAbsIoR<#group_type, #size> },
                RegisterLocation::Relative(_) => quote! { RegisterRelIoR<#group_type, #size> },
            };
            r.push(read);
        }

        if let AccessMode::Write | AccessMode::ReadWrite = self.access_mode {
            let write = match &self.write_location {
                RegisterLocation::Index(_) => quote! { RegisterIndexIoW<#group_type, #size> },
                RegisterLocation::Absolute(_) => quote! { RegisterAbsIoW<#group_type, #size> },
                RegisterLocation::Relative(_) => quote! { RegisterRelIoW<#group_type, #size> },
            };
            r.push(write);
        }

        r
    }

    fn contains_reserved_bit_fields(&self) -> bool {
        for bit_field in &self.functions {
            if let FunctionStatus::Reserved = &bit_field.status {
                return true;
            }
        }

        false
    }
}

fn register_struct(r: &Register, group_type: &Ident) -> TokenStream {
    let name = r.register_rust_name();
    let io_traits = r.io_traits_rust(group_type);
    let type_bound = quote! { #( #io_traits )+* };
    let doc = r.description_rust();
    quote! {
        #doc
        pub struct #name<'a, T: #type_bound> {
            io: &'a mut T,
        }
    }
}

fn location_trait(r: &Register, rd: &RegisterDescription, group_type: &Ident, location: RegisterLocation, const_postfix: &str, trait_postfix: &str) -> (TokenStream, Ident) {
    let name = r.register_rust_name();
    let io_traits = r.io_traits_rust(group_type);
    let type_bounds = quote! { #( #io_traits )+* };
    let index_const_type = ident(rd.index_size.rust_unsigned_integer());
    let address_const_type = rd.address_size.rust_type();

    let (trait_name, const_name, const_type, const_value) = match location {
        RegisterLocation::Index(value) => {
            let const_value = lit_int(value);
            let const_name = ident(format!("INDEX{}", const_postfix));
            let trait_name = ident(format!("LocationIndex{}", trait_postfix));
            (trait_name, const_name, index_const_type, const_value)
        }
        RegisterLocation::Absolute(value) => {
            let const_value = lit_int(value);
            let const_name = ident(format!("ABS_ADDRESS{}", const_postfix));
            let trait_name = ident(format!("LocationAbs{}", trait_postfix));
            (trait_name, const_name, address_const_type, const_value)
        }
        RegisterLocation::Relative(value) => {
            let const_value = lit_int(value);
            let const_name = ident(format!("REL_ADDRESS{}", const_postfix));
            let trait_name = ident(format!("LocationRel{}", trait_postfix));
            (trait_name, const_name, address_const_type, const_value)
        }
    };

    (quote! {
        impl <'a, T: #type_bounds> #trait_name for super::#name<'a, T> {
            const #const_name: #const_type = #const_value;
        }
    }, const_name)
}

fn register_struct_impl(r: &Register, rd: &RegisterDescription, group_type: &Ident) -> TokenStream {
    let name = r.register_rust_name();
    let io_traits = r.io_traits_rust(group_type);
    let type_bounds = quote! { #( #io_traits )+* };

    let (read_location_trait_impl, read_location_const) = location_trait(r, rd, group_type, r.read_location, "_R", "R");
    let (write_location_trait_impl, write_location_const) = location_trait(r, rd, group_type, r.write_location, "_W", "W");

    let mut methods = vec![];

    if let AccessMode::ReadWrite = r.access_mode {
        methods.push(quote! {
            #[doc = "Modifies the contents of the register"]
            #[inline]
            pub fn modify<F>(&mut self, f: F)
            where
                for<'w> F: FnOnce(&R, &'w mut W) -> &'w mut W,
            {
                let r = self.read();
                let mut w = W { raw_bits: r.raw_bits };
                (f)(&r, &mut w);
                self.io.write(Self::#write_location_const, w.raw_bits);
            }
        });
    }

    if let AccessMode::Read | AccessMode::ReadWrite = r.access_mode {
        methods.push(quote! {
            #[doc = "Reads the contents of the register"]
            #[inline]
            pub fn read(&mut self) -> R {
                R { raw_bits: self.io.read(Self::#read_location_const) }
            }
        });
    }

    if let AccessMode::Write | AccessMode::ReadWrite = r.access_mode {
        if !r.contains_reserved_bit_fields() {
            methods.push(quote! {
                #[doc = "Writes to the register"]
                #[inline]
                pub fn write<F>(&mut self, f: F)
                where
                    F: FnOnce(&mut W) -> &mut W,
                {
                    let mut w = W { raw_bits: 0 };
                    (f)(&mut w);
                    self.io.write(Self::#write_location_const, w.raw_bits);
                }
            });
        }
    }

    let location_trait_impl = match r.access_mode {
        AccessMode::Write => quote! { #write_location_trait_impl },
        AccessMode::Read => quote! { #read_location_trait_impl },
        AccessMode::ReadWrite => quote! {
            #read_location_trait_impl
            #write_location_trait_impl
        },
    };

    quote! {
        use super::super::register_trait::*;
        use super::#group_type;

        #location_trait_impl

        impl <'a, T: #type_bounds> InGroup for super::#name<'a, T> {
            type Group = #group_type;
        }

        impl <'a, T: #type_bounds> super::#name<'a, T> {
            pub fn new(io: &'a mut T) -> Self {
                Self { io }
            }

            #( #methods )*
        }
    }
}


fn register_module(r: &Register) -> TokenStream {
    let mut module_code: Vec<TokenStream> = vec![];

    let bit_fields_and_enums = bit_fields_and_enums(r);

    match r.access_mode {
        AccessMode::Read => {
            module_code.push(read_register_code(&r, &bit_fields_and_enums));
        },
        AccessMode::Write => {
            module_code.push(write_register_code(&r, &bit_fields_and_enums));
        },
        AccessMode::ReadWrite => {
            module_code.push(read_register_code(&r, &bit_fields_and_enums));
            module_code.push(write_register_code(&r, &bit_fields_and_enums));
        }
    }

    quote! {
        #( #module_code )*
    }
}

fn read_register_code(r: &Register, bit_fields: &Vec<RegisterBitFieldAndEnum>) -> TokenStream {
    let size = ident(r.size_in_bits.rust_unsigned_integer());

    let mut r_methods: Vec<TokenStream> = vec![];
    let mut r_items: Vec<TokenStream> = vec![];
    let mut r_debug: Vec<TokenStream> = vec![];

    for bit_field in bit_fields {
        r_items.push(bit_field.read_code(&size));

        let r_type = bit_field.read_enum_name();
        let getter = bit_field.snake_case_name();
        let doc = bit_field.description_rust();
        r_methods.push(quote!(
            #doc
            #[inline]
            pub fn #getter(&self) -> #r_type {
                #r_type::from_register_value(self.raw_bits)
            }
        ));

        let field = bit_field.snake_case_name_string();
        let value = match bit_field.enum_type() {
            EnumType::Complete => quote! { &format_args!("{:?}", self.#getter()) },
            EnumType::ReservedBoolean => quote! { &self.#getter().bit() },
            EnumType::ReservedNumber => quote! { &self.#getter().bits() },
        };

        r_debug.push(quote! {
            .field(#field, #value)
        });
    }

    let register = r.name.as_str().to_constant_case();

    quote! {
        #[doc = "Value to write to the register"]
        pub struct R {
            raw_bits: #size,
        }

        impl core::fmt::Debug for R {
            fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
                fmt.debug_struct(#register)
                    #( #r_debug )*
                    .finish()
            }
        }

        impl R {
            #( #r_methods )*
        }

        #( #r_items )*
    }
}

fn write_register_code(r: &Register, bit_fields: &Vec<RegisterBitFieldAndEnum>) -> TokenStream {
    let size = ident(r.size_in_bits.rust_unsigned_integer());

    let mut w_methods: Vec<TokenStream> = vec![];
    let mut w_items: Vec<TokenStream> = vec![];

    for bit_field in bit_fields {
        w_items.push(bit_field.write_code(&size));

        let w_type = bit_field.w_proxy_name();
        let getter = bit_field.snake_case_name();
        let doc = bit_field.description_rust();
        w_methods.push(quote!(
            #doc
            #[inline]
            pub fn #getter(&mut self) -> #w_type<'_> {
                #w_type { w: self }
            }
        ))
    }

    quote! {
        #[doc = "Value read from the register"]
        pub struct W {
            raw_bits: #size,
        }

        impl W {
            #( #w_methods )*
        }

        #( #w_items )*
    }
}

impl RegisterEnumValue {
    fn variant_rust_name(&self) -> Ident {
        ident(self.name.as_str().to_constant_case())
    }

    fn set_method_rust_name(&self) -> Ident {
        ident(self.name.as_str().to_snake_case())
    }

    fn is_method_rust_name(&self) -> Ident {
        ident(format!("is_{}", self.name.as_str().to_snake_case()))
    }

    fn description_rust(&self) -> TokenStream {
        self.description.as_ref().map(|description| quote!{ #[doc = #description] }).unwrap_or_default()
    }

    fn rust_value(&self) -> LitInt {
        lit_int(self.value)
    }
}

impl RegisterEnum {
    fn description_rust(&self) -> TokenStream {
        self.description.as_ref().map(|description| quote!{ #[doc = #description] }).unwrap_or_default()
    }

    fn enum_variant_description_list(&self) -> Vec<TokenStream> {
        self.values.iter()
            .map(|v| {
                let description = v.description_rust();
                quote! {
                    #description
                }
            }).collect()
    }

    fn enum_variant_list(&self) -> Vec<TokenStream> {
        self.values.iter()
            .map(|v| {
                let variant = v.variant_rust_name();
                quote! {
                    #variant
                }
            }).collect()
    }

    fn enum_variant_value_list(&self) -> Vec<TokenStream> {
        self.values.iter()
            .map(|v| {
                let value = v.rust_value();
                quote! {
                    #value
                }
            }).collect()
    }

    fn is_variant_method_list(&self, e_type: &Ident, enum_type: EnumType) -> Vec<TokenStream> {
        let mut r = vec![];
        for v in &self.values {
            let method_name = v.is_method_rust_name();
            let variant = v.variant_rust_name();
            let value = v.rust_value();
            let value_boolean = v.value == 1;

            let compare = match enum_type {
                EnumType::ReservedNumber => quote!( self.bits() == #value ),
                EnumType::ReservedBoolean => quote!( self.bit() == #value_boolean ),
                EnumType::Complete => quote!( *self == #e_type::#variant ),
            };

            let doc = format!("Checks if the value of the field is `{}`", variant);

            r.push(quote! {
                #[doc = #doc]
                #[inline]
                pub fn #method_name(&self) -> bool {
                    #compare
                }
            })
        }
        r
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnumType {
    ReservedBoolean,
    ReservedNumber,
    Complete,
}

pub struct RegisterBitFieldAndEnum {
    /// Bit field which is not marked as reserved.
    bit_field: RegisterFunction,
    register_enum: Option<RegisterEnum>,
}

impl RegisterBitFieldAndEnum {
    fn description_rust(&self) -> TokenStream {
        let mut doc = if self.bit_field.range.bit_count().get() == 1 {
            format!("Bit {}", self.bit_field.range)
        } else {
            format!("Bits {}", self.bit_field.range)
        };

        if let Some(description) = self.bit_field.description() {
            doc = format!("{} - {}", doc, description);
        }

        quote!{ #[doc = #doc] }
    }

    fn snake_case_name_string(&self) -> String {
        self.bit_field.name().unwrap().to_snake_case()
    }

    fn snake_case_name(&self) -> Ident {
        ident(self.bit_field.name().unwrap().to_snake_case())
    }

    fn read_enum_name(&self) -> Ident {
        ident(format!("{}_R", self.bit_field_rust_name()))
    }

    fn write_enum_name(&self) -> Ident {
        ident(format!("{}_W", self.bit_field_rust_name()))
    }

    fn w_proxy_name(&self) -> Ident {
        ident(format!("_{}", self.bit_field_rust_name()))
    }

    fn is_boolean(&self) -> bool {
        self.bit_field.range.bit_count().get() == 1
    }

    fn bit_field_rust_name(&self) -> String {
        self.bit_field.name().unwrap().to_constant_case()
    }

    fn variant_list(&self, register_size: &Ident, only_complete_enum: bool) -> Vec<TokenStream> {
        if let Some(e) = &self.register_enum {
            if e.all_possible_values_are_defined {
                return e.enum_variant_description_list()
                    .iter()
                    .zip(e.enum_variant_list())
                    .zip(e.enum_variant_value_list())
                    .map(|((description, variant), value)| {
                        quote! {
                            #description
                            #variant = #value
                        }
                    })
                    .collect();
            }
        }

        if only_complete_enum {
            return vec![];
        }

        if self.is_boolean() {
            vec![quote! { _Reserved(bool) }]
        } else {
            vec![quote! { _Reserved(#register_size) }]
        }
    }

    fn enum_item(&self, register_size: &Ident, name: &Ident, mode: EnumMode, only_complete_enum: bool) -> TokenStream {
        let variants = self.variant_list(register_size, only_complete_enum);
        let repr = if self.enum_type() == EnumType::Complete {
            quote! { #[repr(#register_size)] }
        } else {
            quote! {}
        };

        let doc = match mode {
            EnumMode::Read => format!("Possible values of the field `{}`", self.snake_case_name()),
            EnumMode::Write => format!("Values that can be written to the field `{}`", self.snake_case_name()),
        };

        let additional_documentation = if let Some(e) = &self.register_enum {
            let doc = e.description_rust();
            quote! {
                #[doc = ""]
                #doc
            }
        } else {
            quote! {}
        };

        quote! {
            #repr
            #[doc = #doc]
            #additional_documentation
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub enum #name {
                #( #variants, )*
            }
        }
    }

    fn bit_field_constants(&self, register_size: &Ident) -> TokenStream {
        let bit_field_max_value = self.bit_field.range.max_value().unwrap();
        let lsb_index = lit_int(self.bit_field.range.lsb);
        let register_mask = lit_int(bit_field_max_value << self.bit_field.range.lsb);

        quote! {
            const _MASK: #register_size = #register_mask;
            const _OFFSET: #register_size = #lsb_index;
        }
    }

    fn conversion_methods(&self, name: &Ident, register_size: &Ident, enum_type: EnumType, only_to_register_value: bool) -> Vec<TokenStream> {
        let remove_additional_bits = quote! {
            let value = value & Self::_MASK;
        };

        let shift_bits_to_index_zero = quote! {
            let value = value >> Self::_OFFSET;
        };

        let shift_bits_to_register_position = quote! {
            let value = value << Self::_OFFSET;
        };

        let mut r = vec![
            self.bit_field_constants(register_size),
        ];

        match enum_type {
            EnumType::Complete => {
                let variants = self.register_enum.as_ref().unwrap().enum_variant_list();
                let values = self.register_enum.as_ref().unwrap().enum_variant_value_list();
                let names = iter::repeat(name);

                r.push(quote! {
                    #[inline]
                    pub fn to_register_value(&self) -> #register_size {
                        let value = *self as #register_size;
                        #shift_bits_to_register_position
                        value
                    }
                });

                if only_to_register_value {
                    return r;
                }

                r.push(quote! {
                    #[inline]
                    pub fn from_register_value(value: #register_size) -> Self {
                        #remove_additional_bits
                        #shift_bits_to_index_zero

                        match value {
                            #( #values => #names::#variants,)*
                            _ => unreachable!(),
                        }
                    }

                    #[doc = "Value of the field as raw bits"]
                    #[inline]
                    pub fn bits(&self) -> #register_size {
                        *self as #register_size
                    }
                });

                if self.is_boolean() {
                    r.push(quote! {
                        #[inline]
                        pub fn bit(&self) -> bool {
                            self.bits() == 1
                        }
                    });
                }
            }
            EnumType::ReservedBoolean => {
                r.push(quote! {
                    #[inline]
                    pub fn to_register_value(&self) -> #register_size {
                        match *self {
                            #name::_Reserved(true) => Self::_MASK,
                            #name::_Reserved(false) => 0,
                        }
                    }
                });

                if only_to_register_value {
                    return r;
                }

                r.push(quote! {
                    #[inline]
                    pub fn from_register_value(value: #register_size) -> Self {
                        #remove_additional_bits

                        #name::_Reserved(value == Self::_MASK)
                    }

                    #[inline]
                    pub fn bit(&self) -> bool {
                        match *self {
                            #name::_Reserved(value) => value,
                        }
                    }
                });
            },
            EnumType::ReservedNumber => {
                r.push(quote! {
                    #[inline]
                    pub fn to_register_value(&self) -> #register_size {
                        let value = match *self {
                            #name::_Reserved(value) => value,
                        };
                        #shift_bits_to_register_position
                        value
                    }
                });

                if only_to_register_value {
                    return r;
                }

                r.push(quote! {
                    #[inline]
                    pub fn from_register_value(value: #register_size) -> Self {
                        #remove_additional_bits
                        #shift_bits_to_index_zero

                        #name::_Reserved(value)
                    }

                    #[doc = "Value of the field as raw bits"]
                    #[inline]
                    pub fn bits(&self) -> #register_size {
                        match *self {
                            #name::_Reserved(value) => value,
                        }
                    }
                });
            }
        }
        r
    }

    fn enum_type(&self) -> EnumType {
        match (self.is_boolean(), &self.register_enum) {
            (_, Some(e)) if e.all_possible_values_are_defined => EnumType::Complete,
            (false, _) => EnumType::ReservedNumber,
            (true, _) => EnumType::ReservedBoolean,
        }
    }

    fn read_enum_impl(&self, register_size: &Ident) -> TokenStream {
        let name = self.read_enum_name();
        let mut methods = vec![];

        let enum_type = self.enum_type();

        if let Some(e) = &self.register_enum {
            methods.extend(e.is_variant_method_list(&name, enum_type));
        }

        methods.extend(self.conversion_methods(&name, &register_size, enum_type, false));

        if self.is_boolean() {
            methods.push(quote! {
                #[doc = "Returns `false` if the bit is clear (0)"]
                #[inline]
                pub fn bit_is_clear(&self) -> bool {
                    !self.bit()
                }

                #[doc = "Returns `true` if the bit is set (1)"]
                #[inline]
                pub fn bit_is_set(&self) -> bool {
                    self.bit()
                }
            });
        }

        quote! {
            impl #name {
                #( #methods )*
            }
        }
    }


    fn read_code(&self, register_size: &Ident) -> TokenStream {

        let e = self.enum_item(register_size, &self.read_enum_name(), EnumMode::Read, false);
        let e_impl = self.read_enum_impl(register_size);

        quote! {
            #e
            #e_impl
        }
    }

    fn w_proxy_methods(&self, register_size: &Ident) -> Vec<TokenStream> {
        let w_enum_name = self.write_enum_name();

        let mut r = vec![
            self.bit_field_constants(register_size),
        ];

        if self.is_boolean() {
            r.push(quote! {
                #[doc = "Sets the field bit"]
                #[inline]
                pub fn set_bit(self) -> &'a mut W {
                    self.bit(true)
                }

                #[doc = "Clears the field bit"]
                #[inline]
                pub fn clear_bit(self) -> &'a mut W {
                    self.bit(false)
                }

                #[doc = "Writes raw bits to the field"]
                #[inline]
                pub fn bit(self, value: bool) -> &'a mut W {
                    if value {
                        // Set register bit.
                        self.w.raw_bits |= Self::_MASK;
                    } else {
                        // Clear register bit.
                        self.w.raw_bits &= !Self::_MASK;
                    }
                    self.w
                }
            })
        } else {
            r.push(quote! {
                #[doc = "Writes raw bits to the field"]
                #[inline]
                pub fn bits(self, value: #register_size) -> &'a mut W {
                    // Convert bit field value to register value.
                    let value = value << Self::_OFFSET;
                    // Clear other bits which are not part of this bit field.
                    let value = value & Self::_MASK;

                    // Clear old bit field value from the register.
                    self.w.raw_bits &= !Self::_MASK;
                    // Update new bit field value to the register.
                    self.w.raw_bits |= value;
                    self.w
                }
            })
        }

        if self.enum_type() == EnumType::Complete {
            r.push(quote! {
                #[doc = "Writes `variant` to the field"]
                #[inline]
                pub fn variant(self, variant: #w_enum_name) -> &'a mut W {
                    // Clear old bit field value from the register.
                    self.w.raw_bits &= !Self::_MASK;
                    // Update new bit field value to the register.
                    self.w.raw_bits |= variant.to_register_value();
                    self.w
                }
            });

            for e in &self.register_enum.as_ref().unwrap().values {
                let name = e.set_method_rust_name();
                let variant_name = e.variant_rust_name();
                let doc = e.description_rust();
                r.push(quote! {
                    #doc
                    #[inline]
                    pub fn #name(self) -> &'a mut W {
                        self.variant(#w_enum_name::#variant_name)
                    }
                });
            }
        } else {
            if let Some(e) = &self.register_enum {
                for v in &e.values {
                    let name = v.set_method_rust_name();
                    let variant_value = v.rust_value();
                    let doc = v.description_rust();
                    r.push(quote! {
                        #doc
                        #[inline]
                        pub fn #name(self) -> &'a mut W {
                            // Convert bit field value to register value.
                            let value = #variant_value << Self::_OFFSET;

                            // There is no need to clear additional bits from the value
                            // because validator checks from the enum definition that
                            // the value doesn't overflow the bit field.

                            // Clear old bit field value from the register.
                            self.w.raw_bits &= !Self::_MASK;
                            // Update new bit field value to the register.
                            self.w.raw_bits |= value;
                            self.w
                        }
                    });
                }
            }
        }

        r
    }

    fn w_proxy(&self, register_size: &Ident) -> TokenStream {
        let name = self.w_proxy_name();
        let methods = self.w_proxy_methods(register_size);

        quote! {
            #[doc = "Proxy"]
            pub struct #name<'a> {
                w: &'a mut W,
            }

            impl <'a> #name<'a> {
                #( #methods )*
            }
        }
    }

    fn write_code(&self, register_size: &Ident) -> TokenStream {
        let enum_type = self.enum_type();

        let w_enum = if enum_type == EnumType::Complete {
            let e = self.enum_item(register_size, &self.write_enum_name(), EnumMode::Write, true);
            let name = self.write_enum_name();
            let e_methods = self.conversion_methods(&name, &register_size, enum_type, true);

            quote! {
                #e

                impl #name {
                    #( #e_methods )*
                }
            }
        } else {
            quote! {}
        };

        let w_proxy = self.w_proxy(register_size);

        quote! {
            #w_enum
            #w_proxy
        }
    }
}

fn bit_fields_and_enums(r: &Register) -> Vec<RegisterBitFieldAndEnum> {
    r.functions.iter().filter(|bit_field| bit_field.status.is_normal()).map(|bit_field| {
        let mut register_enum = None;
        for e in r.enums.iter() {
            if e.range == bit_field.range {
                register_enum = Some(e.clone());
                break;
            }
        }

        RegisterBitFieldAndEnum {
            bit_field: bit_field.clone(),
            register_enum,
        }
    }).collect()
}

pub enum EnumMode {
    Write,
    Read,
}
