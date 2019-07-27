
use std::{
    convert::TryFrom,
    fmt,
};

use toml::Value;

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


#[derive(Debug)]
pub struct RegisterEnumValue {
    value: u64,
    name: Name,
    description: Option<String>,
}

#[derive(Debug)]
pub struct RegisterEnum {
    name: Name,
    range: BitRange,
    values: Vec<RegisterEnumValue>,
    description: Option<String>,
}

#[derive(Debug, PartialEq)]
/// `self.msb >= self.lsb`
pub struct BitRange {
    msb: u16,
    lsb: u16,
}

impl fmt::Display for BitRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.msb, self.lsb)
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
                    Ok(BitRange {
                        msb,
                        lsb
                    })
                }
            }
            (Some(single_bit), None, None) => {
                let bit: u16 = single_bit.parse::<u16>().map_err(|e| e.to_string())?;

                Ok(BitRange {
                    msb: bit,
                    lsb: bit,
                })
            }
            (_, _, Some(_)) => Err(format!("invalid bit range '{}'", &value)),
            (None, _, None) => unreachable!(), // Iterator method 'next' should make this impossible to happen.
        }
    }
}

#[derive(Debug)]
pub enum FunctionStatus {
    Reserved,
    Normal { name: Name, description: Option<String> },
}

#[derive(Debug)]
pub struct RegisterFunction {
    range: BitRange,
    status: FunctionStatus,
}

#[derive(Debug)]
pub enum AccessMode {
    Read,
    Write,
    ReadWrite,
}

#[derive(Debug)]
pub struct Register {
    name: Name,
    address: u64,
    access_mode: AccessMode,
    size_in_bits: u16,
    alternative_address: Option<u64>,
    description: Option<String>,
    functions: Vec<RegisterFunction>,
    enums: Vec<RegisterEnum>,
    index: Option<u16>,
}


const NAME_KEY: &str = "name";
const DESCRIPTION_KEY: &str = "description";
const BIT_KEY: &str = "bit";
const READ_ADDRESS_KEY: &str = "read_address";
const WRITE_ADDRESS_KEY: &str = "write_address";
const READ_WRITE_ADDRESS_KEY: &str = "read_write_address";
const FUNCTIONS_KEY: &str = "functions";
const RESERVED_KEY: &str = "reserved";
const VALUES_KEY: &str = "values";
const VALUE_KEY: &str = "value";
const ENUMS_KEY: &str = "enums";
const INDEX_KEY: &str = "index";
const SIZE_IN_BITS_KEY: &str = "size_in_bits";

const POSSIBLE_KEYS_REGISTER: &[&str] = &[NAME_KEY, DESCRIPTION_KEY, READ_ADDRESS_KEY, WRITE_ADDRESS_KEY, READ_WRITE_ADDRESS_KEY, FUNCTIONS_KEY, ENUMS_KEY, SIZE_IN_BITS_KEY];
const POSSIBLE_KEYS_FUNCTION: &[&str] = &[BIT_KEY, NAME_KEY, DESCRIPTION_KEY, RESERVED_KEY];
const POSSIBLE_KEYS_ENUM: &[&str] = &[NAME_KEY, BIT_KEY, DESCRIPTION_KEY, VALUES_KEY];
const POSSIBLE_KEYS_ENUM_VALUE: &[&str] = &[VALUE_KEY, NAME_KEY, DESCRIPTION_KEY];


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

    let (access_mode, address, alternative_address) = {
        let read_address = v.value(READ_ADDRESS_KEY).optional()?;
        let write_address = v.value(WRITE_ADDRESS_KEY).optional()?;
        let read_write_address = v.value(READ_WRITE_ADDRESS_KEY).optional()?;

        let (access_mode, value) = match (read_address, write_address, read_write_address) {
            (Some(v), None, None) => (AccessMode::Read, v),
            (None, Some(v), None) => (AccessMode::Write, v),
            (None, None, Some(v)) => (AccessMode::ReadWrite, v),
            (None, None, None) => return v.table_validation_error(format!("register access mode '{}', '{}', or '{}' is required", READ_ADDRESS_KEY, WRITE_ADDRESS_KEY, READ_WRITE_ADDRESS_KEY)),
            _ => return v.table_validation_error(format!("access mode count error: only one access mode is supported")),
        };

        match (&rd.extension, value) {
            (_, Value::Integer(integer)) => {
                if *integer < 0 {
                    return v.table_validation_error(format!("address can't be negative, found: '{}'", integer));
                } else {
                    (access_mode, *integer as u64, None)
                }
            }
            (Some(Extension::Vga), Value::String(number)) => {
                if number.matches("?").count() == 1 && number.starts_with("0x")  {
                    let number_with_hex_b = number.replace("?", "B");
                    let number_with_hex_d = number.replace("?", "D");

                    let address1: u64 = v.hex_to_u64(&number_with_hex_b)?;
                    let address2: u64 = v.hex_to_u64(&number_with_hex_d)?;

                    (access_mode, address1, Some(address2))
                } else {
                    return v.table_validation_error(format!("invalid address '{}', if address is string it must contain one question mark and start with '0x'", &number));
                }
            }
            (_, value) => {
                return v.table_validation_error(format!("unexpected type {:?}", value));
            }
        }
    };

    let size_in_bits = v.u16(SIZE_IN_BITS_KEY).optional()?.or(rd.default_register_size_in_bits);
    let size_in_bits = match size_in_bits {
        Some(size) => size,
        None => return v.table_validation_error(format!("register size is undefined")),
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

    Ok(Register {
        name,
        address,
        access_mode,
        size_in_bits,
        alternative_address,
        description,
        functions,
        enums,
        index,
    })
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
        values
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

    let value = v.integer(VALUE_KEY).require()? as u64;
    let description = v.string(DESCRIPTION_KEY).optional()?;

    Ok(RegisterEnumValue {
        value,
        name,
        description,
    })
}
