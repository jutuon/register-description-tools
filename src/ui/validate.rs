
use std::fs;

use cursive::{
    Cursive,
    views::{
        TextView,
        Dialog,
    },
    traits::*,
};

use crate::logic::{
    validation::{
        self,
        ParsedFile,
        Registers,
        register::{
            AccessMode,
            RegisterSize,
            RegisterLocation,
        },
    },
};

use super::{
    object::UiRegister,
    field::StringField,
};


pub fn convert_to_toml(register: &UiRegister, register_file: &ParsedFile) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    let group = match &register_file.registers {
        Some(Registers::Groups(_)) => format!(".{}", register.group.value.trim()),
        _ => String::new(),
    };

    writeln!(output, "\n[[register{}]]", group).unwrap();
    string_field(&mut output, "name", &register.name);
    string_field(&mut output, "description", &register.description);

    match &register.location_mode.value {
        RegisterLocation::Index(_) => number_or_boolean_field(&mut output, "index", &register.location.value),
        RegisterLocation::Relative(_) => number_or_boolean_field(&mut output, "relative_address", &register.location.value),
        RegisterLocation::Absolute(_) => number_or_boolean_field(&mut output, "absolute_address", &register.location.value),
    }

    if let Some(default_access) = register_file.description.default_register_access {
        if default_access != register.access.value {
            register_access_field(&mut output, register.access.value)
        }
    } else {
        register_access_field(&mut output, register.access.value)
    }

    if let Some(default_size) = register_file.description.default_register_size_in_bits {
        if default_size != register.size.value {
            register_size_field(&mut output, register.size.value)
        }
    } else {
        register_size_field(&mut output, register.size.value)
    }

    writeln!(output, "bit_fields = [").unwrap();
    for f in &register.functions {
        write!(output, "    {{ bit = \"{}\"", f.bit.value.trim()).unwrap();
        if f.reserved.value {
            write!(output, ", reserved = true").unwrap();
        } else {
            write!(output, ", name = \"{}\"", f.name.value.trim()).unwrap();
        }

        let description = f.description.value.trim();
        if description.len() != 0 {
            write!(output, ", description = \"{}\"", description).unwrap();
        }

        writeln!(output, " }},").unwrap();
    }
    writeln!(output, "]").unwrap();

    for e in &register.enums {
        writeln!(output, "\n[[register{}.enum]]", group).unwrap();
        string_field(&mut output, "name", &e.name);
        string_field(&mut output, "description", &e.description);
        string_field(&mut output, "bit", &e.bit);
        writeln!(output, "values = [").unwrap();
        for v in &e.values {
            write!(output, "    {{ value = {}", v.value.value.trim()).unwrap();
            write!(output, ", name = \"{}\"", v.name.value.trim()).unwrap();
            let description = v.description.value.trim();
            if description.len() != 0 {
                write!(output, ", description = \"{}\"", description).unwrap();
            }
            writeln!(output, " }},").unwrap();
        }
        writeln!(output, "]").unwrap();
    }

    output
}

pub fn validate_and_save_ui_register(
    s: &mut Cursive,
    register: &UiRegister,
    register_file: &ParsedFile,
    raw_register_file: &mut String,
    file_path: &str,
) -> Result<(), ()> {
    let new_toml = convert_to_toml(register, register_file);

    let mut new_register_file = raw_register_file.to_string();
    new_register_file.push_str(&new_toml);

    let root_table: toml::value::Table = error_message_and_string(s, toml::from_str(&new_register_file).map_err(|e| e.to_string()), &new_toml)?;
    let r = validation::check_root_table(root_table);

    if let Err(errors) = &r {
        use std::fmt::Write;

        let mut error_string = String::new();
        for e in errors {
            writeln!(error_string, "{}\n", e).unwrap();
        }

        error_message_and_string(s, Err(error_string), &new_toml)?;
    }

    use std::io::Write;

    let mut file = error_message(s, fs::OpenOptions::new().append(true).open(file_path))?;
    error_message(s, file.write_all(new_toml.as_bytes()))?;

    *raw_register_file = new_register_file;

    Ok(())
}

fn string_field(file: &mut String, key: &str, field: &StringField) {
    use std::fmt::Write;

    let value = field.value.trim();
    if value.len() != 0 {
        writeln!(file, "{} = \"{}\"", key, value).unwrap();
    }
}

fn number_or_boolean_field(file: &mut String, key: &str, value: &str) {
    use std::fmt::Write;

    let value = value.trim();
    if value.len() != 0 {
        writeln!(file, "{} = {}", key, value).unwrap();
    }
}

fn register_size_field(file: &mut String, value: RegisterSize) {
    number_or_boolean_field(file, "size", &value.to_string())
}

fn register_access_field(file: &mut String, value: AccessMode) {
    use std::fmt::Write;
    writeln!(file, "{} = \"{}\"", "access", value.to_string()).unwrap();
}

fn error_message<T, U: ToString>(s: &mut Cursive, result: Result<T, U>) -> Result<T, ()> {
    result.map_err(|e| {
        let text = TextView::new(e.to_string()).scrollable();
        let d = Dialog::new()
            .content(text)
            .title("Error")
            .button("Close", |s| { s.pop_layer(); });
        s.add_layer(d);
    })
}

fn error_message_and_string<T, U: ToString>(s: &mut Cursive, result: Result<T, U>, text: &str) -> Result<T, ()> {
    error_message(s, result.map_err(|e| {
        let mut error = e.to_string();
        error.push_str("\n");
        error.push_str(text);
        error
    }))
}
