
use std::{
    convert::TryFrom,
    collections::HashMap,
    num::NonZeroU32,
    fmt,
};

use super::{
    CurrentTable,
    ParserContextAndErrors,
    TableValidator,
    TomlTable,
    Name,
    register_description::{
        RegisterDescription,
        Extension,
    },
};


#[derive(Debug, PartialEq, Copy, Clone)]
pub enum RegisterSize {
    Size8 = 8,
    Size16 = 16,
    Size32 = 32,
    Size64 = 64,
}

impl fmt::Display for RegisterSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", *self as usize)
    }
}

impl TryFrom<&str> for RegisterSize {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let s = match value {
            "8"  => RegisterSize::Size8,
            "16" => RegisterSize::Size16,
            "32" => RegisterSize::Size32,
            "64" => RegisterSize::Size64,
            size => {
                return Err(format!("unsupported register size {}, supported register sizes are 8, 16, 32 and 64", size))
            }
        };

        Ok(s)
    }
}

impl TryFrom<usize> for RegisterSize {
    type Error = String;
    /// Value is enum variant index or register size.
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 | 8 => RegisterSize::Size8,
            1 | 16 => RegisterSize::Size16,
            2 | 32 => RegisterSize::Size32,
            3 | 64 => RegisterSize::Size64,
            _ => return Err(format!("can't convert value {} to RegisterSize enum", value)),
        })
    }
}

#[derive(Debug, Clone)]
pub struct RegisterEnumValue {
    pub value: u64,
    pub name: Name,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegisterEnum {
    pub name: Name,
    pub range: BitRange,
    pub values: Vec<RegisterEnumValue>,
    pub description: Option<String>,
    pub all_possible_values_are_defined: bool,
}

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
/// `self.msb >= self.lsb`
pub struct BitRange {
    pub msb: u16,
    pub lsb: u16,
}

impl BitRange {
    /// Panics if msb or lsb is not valid.
    pub fn new(msb: u16, lsb: u16) -> Self {
        if msb >= lsb {
            Self {
                msb,
                lsb
            }
        } else {
            panic!("error: msb < lsb, msb: {}, lsb: {}");
        }
    }

    pub fn bit_count(&self) -> NonZeroU32 {
        let msb = self.msb as u32;
        let lsb = self.lsb as u32;

        // 0 - 0 + 1 = 1
        // 1 - 0 + 1 = 2
        // 2 - 0 + 1 = 3
        // 2 - 1 + 1 = 2
        NonZeroU32::new(msb - lsb + 1).unwrap()
    }

    /// Returns error if bit range is larger than 64 bits.
    pub fn max_value(&self) -> Result<u64, String> {
        let bit_count = self.bit_count();
        if bit_count.get() > 64 {
            return Err(format!("bit range '{}' is larger than 64 bits", self));
        }

        let max_value = if bit_count.get() == 64 {
            u64::max_value()
        } else {
            2u64.pow(bit_count.get()) - 1
        };

        Ok(max_value)
    }
}

impl fmt::Display for BitRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.msb == self.lsb {
            write!(f, "{}", self.msb)
        } else {
            write!(f, "{}:{}", self.msb, self.lsb)
        }
    }
}

impl TryFrom<&str> for BitRange {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut bit = value.split(":");
        match (bit.next(), bit.next(), bit.next()) {
            (Some(msb), Some(lsb), None) => {
                let msb = msb.parse::<u16>().map_err(|e| e.to_string())?;
                let lsb = lsb.parse::<u16>().map_err(|e| e.to_string())?;

                if msb < lsb {
                    Err(format!("most significant bit is smaller than least significant bit (msb < lsb), value: '{}'", &value))
                } else if msb == lsb {
                    Err(format!("unnecessary range syntax, change '{}' to '{}'", &value, msb))
                } else {
                    Ok(BitRange::new(msb, lsb))
                }
            }
            (Some(single_bit), None, None) => {
                let bit: u16 = single_bit.parse::<u16>().map_err(|e| e.to_string())?;

                Ok(BitRange::new(bit, bit))
            }
            (_, _, Some(_)) => Err(format!("invalid bit range '{}'", &value)),
            (None, _, None) => unreachable!(), // Iterator method 'next' should make this impossible to happen.
        }
    }
}

#[derive(Debug, Clone)]
pub enum FunctionStatus {
    Reserved,
    Normal { name: Name, description: Option<String> },
}

impl FunctionStatus {
    pub fn is_reserved(&self) -> bool {
        if let FunctionStatus::Reserved = self {
            true
        } else {
            false
        }
    }

    pub fn is_normal(&self) -> bool {
        !self.is_reserved()
    }
}

#[derive(Debug, Clone)]
pub struct RegisterFunction {
    pub range: BitRange,
    pub status: FunctionStatus,
}

impl RegisterFunction {
    pub fn name(&self) -> Option<&str> {
        if let FunctionStatus::Normal { name, ..} = &self.status {
            Some(name.as_str())
        } else {
            None
        }
    }

    pub fn description(&self) -> Option<&str> {
        if let FunctionStatus::Normal { description, ..} = &self.status {
            description.as_ref().map(|x| x.as_str())
        } else {
            None
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AccessMode {
    Read = 0,
    Write = 1,
    ReadWrite = 2,
}

impl TryFrom<usize> for AccessMode {
    type Error = String;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => AccessMode::Read,
            1 => AccessMode::Write,
            2 => AccessMode::ReadWrite,
            _ => return Err(format!("can't convert value {} to AccessMode enum", value)),
        })
    }
}

impl TryFrom<&str> for AccessMode {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "r" => AccessMode::Read,
            "w" => AccessMode::Write,
            "rw" => AccessMode::ReadWrite,
            _ => return Err(format!("unsupported register access mode '{}', supported modes are 'r', 'w' or 'rw'", value)),
        })
    }
}

impl fmt::Display for AccessMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            AccessMode::Read => "r",
            AccessMode::Write => "w",
            AccessMode::ReadWrite => "rw",
        };

        write!(f, "{}", value)
    }
}

#[derive(Debug)]
pub struct Register {
    pub name: Name,
    pub access_mode: AccessMode,
    pub size_in_bits: RegisterSize,
    pub location: RegisterLocation,
    pub description: Option<String>,
    pub functions: Vec<RegisterFunction>,
    pub enums: Vec<RegisterEnum>,
    pub index: Option<u16>,
}

impl Register {
    /// Checks the following properties:
    /// * Function ranges are within register bounds.
    /// * Function ranges do not overlap.
    /// * Function ranges fill the register completely.
    fn check_functions(&self, v: &mut TableValidator<'_,'_>) {
        let mut bits: Vec<Option<&BitRange>> = vec![None; self.size_in_bits as usize];
        for f in self.functions.iter() {
            let mut overlap_detected = false;

            for i in f.range.lsb..=f.range.msb {
                match bits.get_mut(i as usize) {
                    Some(bit @ None) => *bit = Some(&f.range),
                    Some(Some(error_another_function_overlaps)) => {
                        if !overlap_detected {
                            let _ = v.table_validation_error::<()>(format!("function bit range '{}' overlaps with another function '{}'", f.range, error_another_function_overlaps));
                        }
                        overlap_detected = true;

                        // Breaking the loop here can break the undefined register bit check.
                    }
                    None => {
                        let _ = v.table_validation_error::<()>(format!("function bit range '{}' is not inside register bounds, register size: {}", f.range, self.size_in_bits));
                        break;
                    },
                }
            }
        }

        let mut ranges_without_function: Vec<BitRange> = vec![];
        let mut lsb: Option<u16> = None;
        for (i, b) in bits.iter().enumerate() {
            match (lsb, b) {
                // 0111_1001
                //         ^ i
                (None, Some(_)) => (),

                // 0111_1001
                //        ^  i
                (None, None) => lsb = Some(i as u16),

                // 0111_1001
                //       ^   i
                (Some(_), None) => (),

                // 0111_1001
                //      ^    i
                (Some(lsb_value), Some(_)) => {
                    ranges_without_function.push(BitRange::new(
                        (i-1) as u16,
                        lsb_value,
                    ));
                    lsb = None;
                }
            }
        }

        //  0111_1001
        // ^          i
        if let Some(lsb) = lsb {
            ranges_without_function.push(BitRange::new((bits.len() - 1) as u16, lsb));
        }

        match ranges_without_function.len() {
            0 => (),
            1 => {
                let range = &ranges_without_function[0];
                if range.msb == range.lsb {
                    let _ = v.table_validation_error::<()>(format!("register bit '{}' is undefined", range));
                } else {
                    let _ = v.table_validation_error::<()>(format!("some register bits are undefined, '{}'", range));
                }
            }
            _ => {
                let mut ranges_string = String::new();
                for range in &ranges_without_function {
                    use std::fmt::Write;
                    write!(&mut ranges_string, "'{}', ", range).unwrap();
                }

                ranges_string.pop();
                ranges_string.pop();

                let _ = v.table_validation_error::<()>(format!("some register bits are undefined, {}", ranges_string));
            },
        }
    }

    /// Checks the following properties:
    /// * Register enum bit range matches some register function
    ///   which is not marked as reserved.
    /// * Only one register enum can exist per register function.
    /// * Enum values are within enum bit range bounds.
    /// * There is no duplicate enum values.
    ///
    /// Also sets enum flag `all_possible_values_are_defined` if
    /// there exist enough enum values depending on enum bit range size.
    fn check_register_enums(&mut self, v: &mut TableValidator<'_,'_>) {
        let mut enum_bit_ranges: HashMap<BitRange, &Name> = HashMap::new();

        for e in &mut self.enums {
            let mut some_range_matched = false;
            let mut reserved_function_match = false;
            for f in &self.functions {
                if f.range == e.range {
                    some_range_matched = true;

                    if let FunctionStatus::Reserved = &f.status {
                        reserved_function_match = true;
                    }
                }
            }

            if !some_range_matched {
                let _ = v.table_validation_error::<()>(format!("no matching function bit range found for enum '{}'", e.name));
                continue;
            }

            if reserved_function_match {
                let _ = v.table_validation_error::<()>(format!("enum '{}' bit range is reserved", e.name));
                continue;
            }

            if let Some(another_enum_name) = enum_bit_ranges.insert(e.range, &e.name) {
                let _ = v.table_validation_error::<()>(format!("same bit range '{}' is defined found for enums '{}' and '{}'", e.range, e.name, another_enum_name));
                continue;
            }

            let max_value_for_enum: u64 = match e.range.max_value() {
                Ok(value) => value,
                Err(error) => {
                    let _ = v.table_validation_error::<()>(format!("enum '{}' {}", e.name, error));
                    continue;
                }
            };

            let mut enum_values: HashMap<u64, &Name> = HashMap::new();

            for enum_value in &e.values {
                if enum_value.value > max_value_for_enum {
                    let _ = v.table_validation_error::<()>(format!("enum value '{}' with value '{}' for enum '{}' is larger than enum max value '{}'", enum_value.name, enum_value.value, e.name, max_value_for_enum));
                }

                if let Some(another_name) = enum_values.insert(enum_value.value, &enum_value.name) {
                    let _ = v.table_validation_error::<()>(format!("enum values '{}' and '{}' have the same value '{}'", enum_value.name, another_name, enum_value.value));
                }
            }

            match u128::try_from(e.values.len()) {
                Ok(value_count) => {
                    let required_count = max_value_for_enum as u128 + 1;
                    if required_count == value_count {
                        e.all_possible_values_are_defined = true;
                    }
                }
                Err(error) => {
                    let _ = v.table_validation_error::<()>(format!("validator error: conversion from usize to u128 failed, error: {}", error));
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegisterLocation {
    Index(u64),
    Relative(u64),
    Absolute(u64),
}

impl TryFrom<usize> for RegisterLocation {
    type Error = String;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => RegisterLocation::Index(0),
            1 => RegisterLocation::Relative(0),
            2 => RegisterLocation::Absolute(0),
            _ => return Err(format!("can't convert value {} to RegisterLocation enum", value)),
        })
    }
}


const NAME_KEY: &str = "name";
const DESCRIPTION_KEY: &str = "description";
const BIT_KEY: &str = "bit";
const ACCESS_KEY: &str = "access";
const ABSOLUTE_ADDRESS_KEY: &str = "absolute_address";
const RELATIVE_ADDRESS_KEY: &str = "relative_address";
const FUNCTIONS_KEY: &str = "bit_fields";
const RESERVED_KEY: &str = "reserved";
const VALUES_KEY: &str = "values";
const VALUE_KEY: &str = "value";
const ENUMS_KEY: &str = "enum";
const INDEX_KEY: &str = "index";
const SIZE_IN_BITS_KEY: &str = "size";

const POSSIBLE_KEYS_REGISTER: &[&str] = &[
    NAME_KEY,
    DESCRIPTION_KEY,
    ACCESS_KEY,
    ABSOLUTE_ADDRESS_KEY,
    RELATIVE_ADDRESS_KEY,
    FUNCTIONS_KEY,
    ENUMS_KEY,
    SIZE_IN_BITS_KEY,
    INDEX_KEY,
];

const POSSIBLE_KEYS_FUNCTION: &[&str] = &[
    BIT_KEY,
    NAME_KEY,
    DESCRIPTION_KEY,
    RESERVED_KEY
];

const POSSIBLE_KEYS_ENUM: &[&str] = &[
    NAME_KEY,
    BIT_KEY,
    DESCRIPTION_KEY,
    VALUES_KEY
];

const POSSIBLE_KEYS_ENUM_VALUE: &[&str] = &[
    VALUE_KEY,
    NAME_KEY,
    DESCRIPTION_KEY
];


pub fn validate_register_table(
    table: &TomlTable,
    rd: &RegisterDescription,
    data: &mut ParserContextAndErrors,
) -> Result<Register, ()> {
    let mut v = TableValidator::new(table, CurrentTable::Register, data);

    let name = v.name(NAME_KEY).require()?;
    v.push_context_identifier(format!("register '{}'", name));

    match &rd.extension {
        Some(Extension::Vga) => v.check_unknown_keys(POSSIBLE_KEYS_REGISTER.iter().chain(&[INDEX_KEY])),
        None => v.check_unknown_keys(POSSIBLE_KEYS_REGISTER),
    }

    let description = v.string(DESCRIPTION_KEY).optional()?;

    let location = {
        let index: Option<u64> = v.try_from_integer(INDEX_KEY).optional()?;
        let absolute_address: Option<u64> = v.try_from_integer(ABSOLUTE_ADDRESS_KEY).optional()?;
        let relative_address: Option<u64> = v.try_from_integer(RELATIVE_ADDRESS_KEY).optional()?;

        match (index, absolute_address, relative_address) {
            (Some(v), None, None) => RegisterLocation::Index(v),
            (None, Some(v), None) => RegisterLocation::Absolute(v),
            (None, None, Some(v)) => RegisterLocation::Relative(v),
            (None, None, None) => return v.table_validation_error(format!("register location field '{}', '{}', or '{}' is required", ABSOLUTE_ADDRESS_KEY, RELATIVE_ADDRESS_KEY, INDEX_KEY)),
            _ => return v.table_validation_error(format!("register location field count error: only one location field is supported")),
        }
    };

    let size_in_bits: Option<RegisterSize> = v.try_from_type(SIZE_IN_BITS_KEY).optional()?.or(rd.default_register_size_in_bits);
    let size_in_bits = match size_in_bits {
        Some(size) => size,
        None => return v.table_validation_error(format!("register size is undefined")),
    };

    let access_mode: Option<AccessMode> = v.try_from_type(ACCESS_KEY).optional()?.or(rd.default_register_access);
    let access_mode = match access_mode {
        Some(a) => a,
        None => return v.table_validation_error(format!("register access mode is undefined")),
    };

    let functions = v.array_of_tables(FUNCTIONS_KEY).require()?
        .map(|t| validate_function_table(t, v.data_mut()))
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap())
        .collect();

    let enums = if let Some(iter) = v.array_of_tables(ENUMS_KEY).optional()? {
        iter.map(|t| validate_enum_table(t, v.data_mut()))
            .filter(|r| r.is_ok())
            .map(|r| r.unwrap())
            .collect()
    } else {
        vec![]
    };


    let index = v.u16(INDEX_KEY).optional()?;

    let mut register = Register {
        name,
        location,
        access_mode,
        size_in_bits,
        description,
        functions,
        enums,
        index,
    };

    register.check_functions(&mut v);
    register.check_register_enums(&mut v);

    Ok(register)
}


pub fn validate_function_table(
    table: &TomlTable,
    data: &mut ParserContextAndErrors,
) -> Result<RegisterFunction, ()> {
    let mut v = TableValidator::new(table, CurrentTable::Function, data);

    let bit_range: BitRange = v.try_from_type(BIT_KEY).require()?;
    v.push_context_identifier(format!("function '{}'", bit_range));

    v.check_unknown_keys(POSSIBLE_KEYS_FUNCTION);

    let reserved = v.boolean(RESERVED_KEY).optional()?.unwrap_or(false);
    let name = v.name(NAME_KEY).optional()?;
    let description = v.string(DESCRIPTION_KEY).optional()?;

    let function_status = match (reserved, name) {
        (false, Some(name)) => FunctionStatus::Normal { name, description },
        (false, None) => return v.table_validation_error(format!("missing key '{}'", NAME_KEY)),
        (true, Some(_)) => return v.table_validation_error(format!("key '{}' is not allowed when function is marked as reserved", NAME_KEY)),
        (true, None) => FunctionStatus::Reserved,
    };

    Ok(RegisterFunction {
        range: bit_range,
        status: function_status,
    })
}

pub fn validate_enum_table(
    table: &TomlTable,
    data: &mut ParserContextAndErrors,
) -> Result<RegisterEnum, ()> {
    let mut v = TableValidator::new(table, CurrentTable::Enum, data);

    let name = v.name(NAME_KEY).require()?;
    v.push_context_identifier(format!("enum '{}'", name));

    v.check_unknown_keys(POSSIBLE_KEYS_ENUM);

    let bit_range: BitRange = v.try_from_type(BIT_KEY).require()?;
    let description = v.string(DESCRIPTION_KEY).optional()?;

    let values = v.array_of_tables(VALUES_KEY).require()?
        .map(|t| validate_enum_value_table(t, v.data_mut()))
        .filter(|r| r.is_ok())
        .map(|r| r.unwrap())
        .collect();

    Ok(RegisterEnum {
        name,
        range: bit_range,
        description,
        values,
        all_possible_values_are_defined: false,
    })
}

pub fn validate_enum_value_table(
    table: &TomlTable,
    data: &mut ParserContextAndErrors,
) -> Result<RegisterEnumValue, ()> {
    let mut v = TableValidator::new(table, CurrentTable::EnumValue, data);

    let name = v.name(NAME_KEY).require()?;
    v.push_context_identifier(format!("enum value '{}'", name));

    v.check_unknown_keys(POSSIBLE_KEYS_ENUM_VALUE);

    let value: u64 = v.try_from_integer(VALUE_KEY).require()?;
    let description = v.string(DESCRIPTION_KEY).optional()?;

    Ok(RegisterEnumValue {
        value,
        name,
        description,
    })
}
