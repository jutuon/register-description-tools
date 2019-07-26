
use toml::Value;

use super::{
    ValidationError,
    CurrentTable,
    ErrorContext,
    TableValidator,
    TomlTable,
    register_description::{
        RegisterDescription,
        Extension,
    },
};


#[derive(Debug)]
pub struct RegisterEnumValue {
    value: u64,
    name: String,
    description: Option<String>,
}

#[derive(Debug)]
pub struct RegisterEnum {
    name: String,
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

#[derive(Debug)]
pub enum FunctionStatus {
    Reserved,
    Normal { name: String, description: Option<String> },
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
    name: String,
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
    errors: &mut Vec<ValidationError>,
    registers: &mut Vec<Register>,
) -> Result<(),()> {
    let mut ec = ErrorContext::new(CurrentTable::Register, NAME_KEY, errors);

    match &rd.extension {
        Some(Extension::Vga) => super::check_unknown_keys(table, POSSIBLE_KEYS_REGISTER.iter().chain(&[INDEX_KEY]), &mut ec),
        None => super::check_unknown_keys(table, POSSIBLE_KEYS_REGISTER, &mut ec),
    }

    let mut v = TableValidator::new(table, &mut ec);

    let name = v.string(NAME_KEY).require()?;
    let description = v.string(DESCRIPTION_KEY).optional()?;

    let (access_mode, address, alternative_address) = {
        let read_address = v.value(READ_ADDRESS_KEY).optional()?;
        let write_address = v.value(WRITE_ADDRESS_KEY).optional()?;
        let read_write_address = v.value(READ_WRITE_ADDRESS_KEY).optional()?;

        let (access_mode, value) = match (read_address, write_address, read_write_address) {
            (Some(v), None, None) => (AccessMode::Read, v),
            (None, Some(v), None) => (AccessMode::Write, v),
            (None, None, Some(v)) => (AccessMode::ReadWrite, v),
            (None, None, None) => return v.table_validation_error(format!("Register '{}' doesn't specify access mode.", &name)),
            _ => return v.table_validation_error(format!("Register '{}' access mode count error, only one access mode is supported.", &name)),
        };

        match (&rd.extension, value) {
            (_, Value::Integer(integer)) => {
                if *integer < 0 {
                    return v.table_validation_error(format!("Register '{}' address can't be negative, value: {}", &name, integer));
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
                    return v.table_validation_error(format!("Register '{}' address error. If address is string it must contain one question mark and start with '0x', value: {}", &name, &number));
                }
            }
            (_, value) => {
                return v.table_validation_error(format!("Register '{}' address error: unexpected type {:#?}", &name, value));
            }
        }
    };

    let size_in_bits = v.u16(SIZE_IN_BITS_KEY).optional()?.or(rd.default_register_size_in_bits);
    let size_in_bits = match size_in_bits {
        Some(size) => size,
        None => return v.table_validation_error(format!("Register '{}' size in bits error: size is undefined.", &name)),
    };

    let mut functions = vec![];
    for array in v.array(FUNCTIONS_KEY).require()? {
        match array {
            Value::Table(t) => {
                let _ = validate_function_table(&name, t, v.errors_mut(), &mut functions);
            },
            value => return v.table_validation_error(format!("Register '{}', expected array of tables, value: {:#?}", &name, value)),
        }
    }

    let mut enums = vec![];
    for array in v.array(ENUMS_KEY).optional()?.map(|x| x.as_slice()).unwrap_or_default() {
        match array {
            Value::Table(t) => {
                let _ = validate_enum_table(t, v.errors_mut(), &mut enums);
            },
            value => return v.table_validation_error(format!("Register '{}', expected array of tables, value: {:#?}", &name, value)),
        }
    }

    let index = v.u16(INDEX_KEY).optional()?;

    let register = Register {
        name,
        address,
        access_mode,
        size_in_bits,
        alternative_address,
        description,
        functions,
        enums,
        index,
    };

    registers.push(register);

    Ok(())
}


pub fn validate_function_table(
    register_name: &str,
    table: &TomlTable,
    errors: &mut Vec<ValidationError>,
    functions: &mut Vec<RegisterFunction>,
) -> Result<(),()> {
    let mut ec = ErrorContext::new(CurrentTable::Function, NAME_KEY, errors);
    super::check_unknown_keys(table, POSSIBLE_KEYS_FUNCTION, &mut ec);

    let mut v = TableValidator::new(table, &mut ec);

    let bit_string = v.string(BIT_KEY).require()?;
    let bit_range = validate_bit_range(&bit_string, &mut v)?;

    let reserved = v.boolean(RESERVED_KEY).optional()?.unwrap_or(false);
    let name = v.string(NAME_KEY).optional()?;
    let description = v.string(DESCRIPTION_KEY).optional()?;

    let function_status = match (reserved, name) {
        (false, Some(name)) => FunctionStatus::Normal { name, description },
        (false, None) => return v.table_validation_error(format!("Register '{}', function with bit range '{}': key 'name' is required.", register_name, &bit_string)),
        (true, Some(_)) => return v.table_validation_error(format!("Register '{}', function with bit range '{}': key 'name' is not allowed when function is marked as reserved.", register_name, &bit_string)),
        (true, None) => FunctionStatus::Reserved,
    };

    let function = RegisterFunction {
        range: bit_range,
        status: function_status,
    };

    functions.push(function);

    Ok(())
}

fn validate_bit_range(bit_string: &str, v: &mut TableValidator<'_,'_>) -> Result<BitRange, ()> {
    let mut bit = bit_string.split(":");
    let bit_range = match (bit.next(), bit.next(), bit.next()) {
        (Some(msb), Some(lsb), None) => {
            let msb: u16 = v.handle_error(msb.parse())?;
            let lsb: u16 = v.handle_error(lsb.parse())?;

            if msb < lsb {
                return v.table_validation_error(format!("Error: most significant bit is smaller than least significant bit (msb < lsb), value: {}", &bit_string));
            } else {
                BitRange {
                    msb,
                    lsb
                }
            }
        }
        (Some(single_bit), None, None) => {
            let bit: u16 = v.handle_error(single_bit.parse())?;

            BitRange {
                msb: bit,
                lsb: bit,
            }
        }
        (_, _, Some(_)) => return v.table_validation_error(format!("Invalid bit range: {}", &bit_string)),
        (None, _, None) => unreachable!(), // Iterator method 'next' should make this impossible to happen.
    };

    Ok(bit_range)
}


pub fn validate_enum_table(
    table: &TomlTable,
    errors: &mut Vec<ValidationError>,
    enums: &mut Vec<RegisterEnum>,
) -> Result<(),()> {
    let mut ec = ErrorContext::new(CurrentTable::Enum, NAME_KEY, errors);
    super::check_unknown_keys(table, POSSIBLE_KEYS_ENUM, &mut ec);

    let mut v = TableValidator::new(table, &mut ec);

    let bit_string = v.string(BIT_KEY).require()?;
    let bit_range = validate_bit_range(&bit_string, &mut v)?;
    let name = v.string(NAME_KEY).require()?;
    let description = v.string(DESCRIPTION_KEY).optional()?;

    let mut values = vec![];
    for array in v.array(VALUES_KEY).require()? {
        match array {
            Value::Table(t) => {
                let _ = validate_enum_value_table(t, v.errors_mut(), &mut values);
            },
            value => return v.table_validation_error(format!("Enum '{}', expected array of tables, value: {:#?}", &name, value)),
        }
    }

    let register_enum = RegisterEnum {
        name,
        range: bit_range,
        description,
        values
    };

    enums.push(register_enum);

    Ok(())
}

pub fn validate_enum_value_table(
    table: &TomlTable,
    errors: &mut Vec<ValidationError>,
    enum_values: &mut Vec<RegisterEnumValue>,
) -> Result<(),()> {
    let mut ec = ErrorContext::new(CurrentTable::EnumValue, NAME_KEY, errors);
    super::check_unknown_keys(table, POSSIBLE_KEYS_ENUM_VALUE, &mut ec);

    let mut v = TableValidator::new(table, &mut ec);

    let value = v.integer(VALUE_KEY).require()? as u64;
    let name = v.string(NAME_KEY).require()?;
    let description = v.string(DESCRIPTION_KEY).optional()?;

    let enum_value = RegisterEnumValue {
        value,
        name,
        description,
    };

    enum_values.push(enum_value);

    Ok(())
}
