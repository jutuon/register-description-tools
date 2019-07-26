

use super::{
    ValidationError,
    CurrentTable,
    ErrorContext,
    TableValidator,
    TomlTable,
    register_description::RegisterDescription
};

#[derive(Debug)]
pub struct Register {

}


pub fn validate_register_table(
    table: &TomlTable,
    rd: &RegisterDescription,
    errors: &mut Vec<ValidationError>,
    registers: &mut Vec<Register>,
) {
    unimplemented!()
}
