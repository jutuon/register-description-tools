
use std::{
    fmt,
};

use crate::logic::validation::{
    ParsedFile,
    Registers,
    register::{
        AccessMode,
        RegisterSize,
    },
};

use super::field::*;

fn error_if_empty(text: String, key: &str) -> Result<String, String> {
    if text.trim().is_empty() {
        Err(format!("Field '{}' is empty.", key))
    } else {
        Ok(text)
    }
}

pub struct ObjectHandler {
    pub register: UiRegister,
    pub tmp: TempObjects,
}

pub struct TempObjects {
    pub tmp_function: UiFunction,
    pub tmp_enum: UiEnum,
    pub tmp_enum_value: UiEnumValue,
}

impl TempObjects {
    pub fn new() -> Self {
        TempObjects {
            tmp_function: UiFunction::new(),
            tmp_enum: UiEnum::new(),
            tmp_enum_value: UiEnumValue::new(),
        }
    }
}

impl ObjectHandler {
    pub fn new() -> Self {
        ObjectHandler {
            register: UiRegister::new(),
            tmp: TempObjects::new(),
        }
    }
}

pub struct UiRegister {
    pub name: StringField,
    pub address: StringField,
    pub index: StringField,
    pub description: StringField,
    pub group: StringField,
    pub access: EnumField<AccessMode>,
    pub size: EnumField<RegisterSize>,
    pub functions: Vec<UiFunction>,
    pub enums: Vec<UiEnum>,
}

impl UiRegister {
    pub fn new() -> Self {
        let id = "register";
        UiRegister {
            name: StringField::new("name", "", id, Some(error_if_empty)),
            address: StringField::new("address", "", id, Some(error_if_empty)),
            index: StringField::new("index", "", id, None),
            description: StringField::new("description", "", id, None),
            group: StringField::new("group", "", id, Some(error_if_empty)),
            access: EnumField::new("access", AccessMode::ReadWrite, &[2, 0, 1]),
            size: EnumField::new("size", RegisterSize::Size8, &[0, 1, 2, 3]),
            functions: vec![],
            enums: vec![],
        }
    }
}

impl UiObject for UiRegister {
    fn fields(&mut self, parsed_file: &ParsedFile) -> Vec<&mut dyn TuiField> {
        let mut fields: Vec<&mut dyn TuiField> = vec![
            &mut self.name,
            &mut self.address,
            &mut self.index,
            &mut self.description,
        ];

        match &parsed_file.registers {
            Some(Registers::Groups(_)) => fields.push(&mut self.group),
            _ => (),
        }

        fields.push(&mut self.access);
        fields.push(&mut self.size);
        fields
    }
}


#[derive(Clone)]
pub struct UiEnumValue {
    pub value: StringField,
    pub name: StringField,
    pub description: StringField,
}

impl fmt::Display for UiEnumValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.value.value, self.name.value)
    }
}

impl UiEnumValue {
    pub fn new() -> Self {
        let id = "enum_value";
        UiEnumValue {
            value: StringField::new("value", "", id, Some(error_if_empty)),
            name: StringField::new("name", "", id, Some(error_if_empty)),
            description: StringField::new("description", "", id, None),
        }
    }
}

impl UiObject for UiEnumValue {
    fn fields(&mut self, _: &ParsedFile) -> Vec<&mut dyn TuiField> {
        vec![
            &mut self.name,
            &mut self.value,
            &mut self.description,
        ]
    }
}

impl Default for UiEnumValue {
    fn default() -> Self {
        Self::new()
    }
}


#[derive(Clone)]
pub struct UiEnum {
    pub name: StringField,
    pub bit: StringField,
    pub description: StringField,
    pub values: Vec<UiEnumValue>,
}

impl fmt::Display for UiEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.name.value, self.bit.value)
    }
}


impl UiEnum {
    pub fn new() -> Self {
        let id = "enum";
        UiEnum {
            bit: StringField::new("bit", "", id, Some(error_if_empty)),
            name: StringField::new("name", "", id, Some(error_if_empty)),
            description: StringField::new("description", "", id, None),
            values: vec![],
        }
    }
}

impl UiObject for UiEnum {
    fn fields(&mut self, _: &ParsedFile) -> Vec<&mut dyn TuiField> {
        vec![
            &mut self.name,
            &mut self.bit,
            &mut self.description,
        ]
    }
}

impl Default for UiEnum {
    fn default() -> Self {
        Self::new()
    }
}


#[derive(Clone)]
pub struct UiFunction {
    pub bit: StringField,
    pub reserved: BooleanField,
    pub name: StringField,
    pub description: StringField,
}

impl fmt::Display for UiFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.reserved.value {
            write!(f, "{} - reserved", self.bit.value)
        } else {
            write!(f, "{} - {}", self.bit.value, self.name.value)
        }
    }
}

impl UiFunction {
    pub fn new() -> Self {
        let id = "function";
        UiFunction {
            bit: StringField::new("bit", "", id, Some(error_if_empty)),
            reserved: BooleanField::new("reserved", false, id),
            name: StringField::new("name", "", id, None),
            description: StringField::new("description", "", id, None),
        }
    }
}

impl Default for UiFunction {
    fn default() -> Self {
        Self::new()
    }
}


impl UiObject for UiFunction {
    fn fields(&mut self, _: &ParsedFile) -> Vec<&mut dyn TuiField> {
        vec![
            &mut self.bit,
            &mut self.reserved,
            &mut self.name,
            &mut self.description,
        ]
    }
}

pub trait UiObject {
    fn fields(&mut self, parsed_file: &ParsedFile) -> Vec<&mut dyn TuiField>;
}
