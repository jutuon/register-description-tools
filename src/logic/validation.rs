pub mod register_description;
pub mod register;

// TODO: Check name and description values with regex.
// TODO: Check that register function bit ranges don't overlap
//       and are inside register bounds.
// TODO: Check that the same register enum bit range is defined also in the register
//       function list.

use std::{
    convert::TryFrom,
    iter::Iterator,
    fmt,
};

use toml::Value;

use register_description::RegisterDescription;
use register::Register;

pub type TomlTable = toml::value::Table;
pub type TomlValue = toml::value::Value;
pub type TomlArray = toml::value::Array;

#[derive(Debug, Copy, Clone)]
pub enum CurrentTable {
    Root,
    RegisterDescription,
    Register,
    Enum,
    EnumValue,
    Function,
}

#[derive(Debug)]
pub enum ValidationError {
    MissingKey { table: CurrentTable, context: String, key:  &'static str },
    UnknownKey { table: CurrentTable, context: String, key: String },
    /// Type or contents of the value was unexpected.
    ValueValidationError { table: CurrentTable, context: String, key: &'static str, error: String },
    /// Table value is invalid because of other table value.
    ///
    /// For example defining register function to bit 15 when register size is 8 bit
    /// produces this error.
    TableValidationError { table: CurrentTable, context: String, error: String },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::MissingKey { table, context, key} => write!(f, "error: key '{}' is missing from table type '{:?}'{}", key, table, context),
            ValidationError::UnknownKey { table, context, key} => write!(f, "error: unsupported key '{}' in table type '{:?}'{}", key, table, context),
            ValidationError::ValueValidationError { table, context, key, error} => write!(f, "error: {}, key: '{}', table type: '{:?}'{}", error, key, table, context),
            ValidationError::TableValidationError { table, context, error } => write!(f, "error: {}, table type: '{:?}'{}", error, table, context),
        }
    }
}

#[derive(Debug)]
pub struct ParsedFile {
    description: RegisterDescription,
    registers: Option<Registers>,
}

#[derive(Debug)]
pub enum Registers {
    Groups(Vec<(String, Vec<Register>)>),
    OnlyRegisters(Vec<Register>),
}


const REGISTER_DESCRIPTION_KEY: &str = "register_description";
const REGISTER_KEY: &str = "register";
const POSSIBLE_ROOT_KEYS: &[&str] = &[REGISTER_DESCRIPTION_KEY, REGISTER_KEY];

pub fn check_root_table(root: TomlTable) -> Result<ParsedFile, Vec<ValidationError>> {
    let mut data = ParserContextAndErrors::default();

    let mut v = TableValidator::new(&root, CurrentTable::Root, &mut data);
    v.check_unknown_keys(POSSIBLE_ROOT_KEYS);

    let rd = match v.table(REGISTER_DESCRIPTION_KEY).require() {
        Ok(table) => {
            match register_description::check_register_description(table, v.data_mut()) {
                Ok(rd) => rd,
                Err(()) => {
                    drop(v);
                    return Err(data.errors)
                }
            }
        },
        Err(()) => {
            drop(v);
            return Err(data.errors);
        }
    };

    let mut parsed_file = ParsedFile {
        description: rd,
        registers: None,
    };

    match v.value(REGISTER_KEY).optional() {
        Ok(Some(Value::Array(array))) => {
            let registers = handle_register_array(array, &mut v, &parsed_file);
            parsed_file.registers = Some(Registers::OnlyRegisters(registers));
        }
        Ok(Some(Value::Table(table))) => {
            let mut groups = vec![];
            for (key, value) in table.iter() {
                match value {
                    Value::Array(array) => {
                        let registers = handle_register_array(array, &mut v, &parsed_file);
                        groups.push((key.to_string(), registers));
                    },
                    invalid_type => {
                        v.value_validation_error(format!("validating register group '{}' failed: expected an array, found {:?}", key, invalid_type));
                    }
                }
            }
        }
        Ok(Some(invalid_type)) => {
            v.value_validation_error(format!("expected a table or an array, found: {:?}", invalid_type));
        }
        Err(()) | Ok(None) => (),
    }

    drop(v);

    if data.errors.len() == 0 {
        Ok(parsed_file)
    } else {
        Err(data.errors)
    }

}

pub fn handle_register_array(array: &TomlArray, v: &mut TableValidator, parsed_file: &ParsedFile) -> Vec<Register> {
    let mut registers = vec![];
    for value in array {
        match value {
            Value::Table(register_table) => {
                if let Ok(r) = register::validate_register_table(register_table, &parsed_file.description, v.data_mut()) {
                    registers.push(r);
                }
            },
            invalid_type => {
                v.value_validation_error(format!("expected an array of tables, found: {:?},", invalid_type));
            }
        }
    }
    registers
}

#[derive(Default)]
pub struct ParserContextAndErrors {
    context_stack: Vec<String>,
    errors: Vec<ValidationError>,
}

struct ErrorContext<'a> {
    ct: CurrentTable,
    current_key: &'static str,
    context_stack_push_count: usize,
    data: &'a mut ParserContextAndErrors,
}

impl Drop for ErrorContext<'_> {
    fn drop(&mut self) {
        for _ in 0..self.context_stack_push_count {
            self.data.context_stack.pop();
        }
    }
}

impl <'a> ErrorContext<'a> {
    fn new(ct: CurrentTable, data: &'a mut ParserContextAndErrors) -> Self {
        Self {
            ct,
            current_key: "current key is uninitialized",
            context_stack_push_count: 0,
            data,
        }
    }

    fn push_context_identifier(&mut self, text: String) {
        self.context_stack_push_count = self.context_stack_push_count.checked_add(1).expect("ErrorContext self.context_stack_push_count overflowed.");
        self.data.context_stack.push(text);
    }

    fn change_current_key(&mut self, new: &'static str) {
        self.current_key = new;
    }

    /// Add error with current table information.
    fn unknown_key(&mut self, unknown_key: String) {
        self.data.errors.push(ValidationError::UnknownKey {
            table: self.ct,
            context: self.context_stack_string(),
            key: unknown_key,
        });
    }

    /// Add error with current table and current key information.
    fn missing_key(&mut self) {
        self.data.errors.push(ValidationError::MissingKey {
            table: self.ct,
            context: self.context_stack_string(),
            key: self.current_key
        });
    }

    /// Add error with current table and current key information.
    fn value_validation_error(&mut self, error: String) {
        self.data.errors.push(ValidationError::ValueValidationError {
            table: self.ct,
            context: self.context_stack_string(),
            key: self.current_key,
            error,
        });
    }

    /// Add error with current table information.
    fn table_validation_error(&mut self, error: String) {
        self.data.errors.push(ValidationError::TableValidationError{
            table: self.ct,
            context: self.context_stack_string(),
            error
        });
    }

    fn data_mut(&mut self) -> &mut ParserContextAndErrors {
        &mut self.data
    }

    fn context_stack_string(&self) -> String {
        let mut ct = format!("");
        for s in &self.data.context_stack {
            ct.push_str("\n\t--> ");
            ct.push_str(s);
        }
        ct
    }
}

/// Validator closure can assume that item != Item::None.
fn optional_key_check<'a, 'b, T, U: FnMut(&'a TomlValue, &mut ErrorContext) -> Result<T, ()>>(
    table: &'a TomlTable,
    key: &'static str,
    ec: &'b mut ErrorContext,
    mut validator: U,
) -> Result<Option<T>, ()> {
    ec.change_current_key(key);
    match table.get(key) {
        None => Ok(None),
        Some(item) => Ok(Some((validator)(item, ec)?)),
    }
}

pub struct TableValidator<'a, 'b> {
    table: &'a TomlTable,
    ec: ErrorContext<'b>,
}

impl <'a, 'b> TableValidator<'a, 'b> {
    pub fn new(table: &'a TomlTable, ct: CurrentTable, data: &'b mut ParserContextAndErrors) -> Self {
        Self {
            table,
            ec: ErrorContext::new(ct, data),
        }
    }

    pub fn check_unknown_keys<T: AsRef<str>, U: Iterator<Item=T> + Clone, V: IntoIterator<Item=T, IntoIter=U>>(&mut self, possible_keys: V) {
        let possible_keys = possible_keys.into_iter();
        for (k, _) in self.table.iter() {
            let mut possible_keys = possible_keys.clone();
            if possible_keys.find(|key_text| &k.as_str() == &key_text.as_ref()).is_none() {
                self.ec.unknown_key(k.to_string())
            }
        }
    }

    pub fn push_context_identifier(&mut self, text: String) {
        self.ec.push_context_identifier(text);
    }

    pub fn data_mut(&mut self) -> &mut ParserContextAndErrors {
        self.ec.data_mut()
    }

    pub fn value_validation_error(&mut self, message: String) {
        self.ec.value_validation_error(message)
    }

    pub fn handle_error<T, E: ToString>(&mut self, value: Result<T, E>) -> Result<T,()> {
        match value {
            Err(e) => self.table_validation_error(e.to_string()),
            Ok(x) => Ok(x)
        }
    }

    /// Result is for early returns with `?` operator or `return` statement.
    pub fn table_validation_error<T>(&mut self, message: String) -> Result<T,()> {
        self.ec.table_validation_error(message);
        Err(())
    }

    pub fn hex_to_u64(&mut self, hex: &str) -> Result<u64, ()> {
        let hex = hex.trim_start_matches("0x");

        match u64::from_str_radix(hex, 16) {
            Err(e) => {
                self.table_validation_error(format!("{}", e))
            }
            Ok(x) => Ok(x),
        }
    }

    pub fn value<'c>(&'c mut self, key: &'static str) -> ValidatorResult<'c, 'a, 'b, &'a TomlValue> {
        let r = optional_key_check(self.table, key, &mut self.ec, |item, _| Ok(item));
        ValidatorResult(r, self)
    }

    pub fn array<'c>(&'c mut self, key: &'static str) -> ValidatorResult<'c, 'a, 'b, &'a TomlArray> {
        let r = optional_key_check(self.table, key, &mut self.ec, |item, ec| {
            match item.as_array() {
                Some(x) => Ok(x),
                None => {
                    ec.value_validation_error(format!("expected an array, found: {:?}", item));
                    Err(())
                }
            }
        });
        ValidatorResult(r, self)
    }

    pub fn array_of_tables<'c>(&'c mut self, key: &'static str) -> ValidatorResult<'c, 'a, 'b, impl Iterator<Item=&'a TomlTable>> {
        self.array(key).map(|array| {
            match array.first() {
                Some(Value::Table(_)) | None => Ok(array.iter().map(|x| x.as_table().unwrap())),
                Some(_) => {
                    Err(format!("expected an array of tables, found: {:?}", array))
                }
            }
        })
    }

    pub fn table<'c>(&'c mut self, key: &'static str) -> ValidatorResult<'c, 'a, 'b, &'a TomlTable> {
        let r = optional_key_check(self.table, key, &mut self.ec, |item, ec| {
            match item.as_table() {
                Some(x) => Ok(x),
                None => {
                    ec.value_validation_error(format!("expected a table, found: {:?}", item));
                    Err(())
                }
            }
        });
        ValidatorResult(r, self)
    }

    pub fn boolean<'c>(&'c mut self, key: &'static str) -> ValidatorResult<'c, 'a, 'b, bool> {
        let r = optional_key_check(self.table, key, &mut self.ec, |item, ec| {
            match item.as_bool() {
                Some(x) => Ok(x),
                None => {
                    ec.value_validation_error(format!("expected a boolean, found: {:?}", item));
                    Err(())
                }
            }
        });
        ValidatorResult(r, self)
    }

    pub fn integer<'c>(&'c mut self, key: &'static str) -> ValidatorResult<'c, 'a, 'b, i64> {
        let r = optional_key_check(self.table, key, &mut self.ec, |item, ec| {
            match item.as_integer() {
                Some(x) => Ok(x),
                None => {
                    ec.value_validation_error(format!("expected an integer, found: {:?}", item));
                    Err(())
                }
            }
        });
        ValidatorResult(r, self)
    }

    pub fn text<'c>(&'c mut self, key: &'static str) -> ValidatorResult<'c, 'a, 'b, &'a str> {
        let r = optional_key_check(self.table, key, &mut self.ec, |item, ec| {
            match item.as_str() {
                Some(text) => Ok(text),
                None => {
                    ec.value_validation_error(format!("expected a string, found: {:?}", item));
                    Err(())
                }
            }
        });
        ValidatorResult(r, self)
    }

    pub fn u16<'c>(&'c mut self, key: &'static str) -> ValidatorResult<'c, 'a, 'b, u16> {
        self.integer(key).map(|number| {
            if number < 0 {
                Err(format!("negative value '{}'", number))
            } else if number > u16::max_value() as i64 {
                Err(format!("larger value than u16::max_value '{}'", number))
            } else {
                Ok(number as u16)
            }
        })
    }

    pub fn try_from_type<'c, T: TryFrom<&'c str, Error=U>, U: ToString>(&'c mut self, key: &'static str) -> ValidatorResult<'c, 'a, 'b, T> {
        self.text(key).map(|text| {
            T::try_from(text)
        })
    }

    pub fn string<'c>(&'c mut self, key: &'static str) -> ValidatorResult<'c, 'a, 'b, String> {
        self.text(key).map::<_,_,String>(|text| {
            Ok(text.to_string())
        })
    }
}

pub struct ValidatorResult<'a, 'b, 'c, T>(Result<Option<T>, ()>, &'a mut TableValidator<'b, 'c>);

impl <'a, 'b, 'c, T> ValidatorResult<'a,'b,'c, T> {
    pub fn require(self) -> Result<T, ()> {
        match self.0? {
            None => {
                self.1.ec.missing_key();
                Err(())
            }
            Some(x) => Ok(x),
        }
    }

    pub fn optional(self) -> Result<Option<T>, ()> {
        self.0
    }

    fn map<U, V: FnMut(T) -> Result<U, X>, X: ToString>(self, mut converter: V) -> ValidatorResult<'a,'b,'c, U> {
        match self.0 {
            Ok(Some(x)) => match (converter)(x) {
                Ok(new) => ValidatorResult(Ok(Some(new)), self.1),
                Err(e) => {
                    self.1.ec.value_validation_error(e.to_string());
                    ValidatorResult(Err(()), self.1)
                }
            }
            Ok(None) => ValidatorResult(Ok(None), self.1),
            Err(()) => ValidatorResult(Err(()), self.1),
        }
    }
}
