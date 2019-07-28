

use std::{
    convert::TryFrom,
    fmt,
};

use super::{
    CurrentTable,
    TableValidator,
    TomlTable,
    ParserContextAndErrors,
    Name,
    register::RegisterSize,
};

const VERSION_KEY: &str = "version";
const NAME_KEY: &str = "name";
const DESCRIPTION_KEY: &str = "description";
const DEFAULT_REGISTER_SIZE_KEY: &str = "default_register_size_in_bits";
const EXTENSION_KEY: &str = "extension";

const POSSIBLE_KEYS: &[&str] = &[VERSION_KEY, NAME_KEY, DESCRIPTION_KEY, DEFAULT_REGISTER_SIZE_KEY, EXTENSION_KEY];

pub fn check_register_description(table: &TomlTable, data: &mut ParserContextAndErrors) -> Result<RegisterDescription, ()> {
    let mut v = TableValidator::new(table, CurrentTable::RegisterDescription, data);
    v.check_unknown_keys(POSSIBLE_KEYS);

    let name = v.name(NAME_KEY).require()?;
    v.push_context_identifier(format!("register description '{}'", name));
    let version: SpecVersion = v.try_from_type(VERSION_KEY).require()?;


    let description = v.string(DESCRIPTION_KEY).optional()?;
    let extension: Option<Extension> = v.try_from_type(EXTENSION_KEY).optional()?;
    let default_register_size_in_bits: Option<RegisterSize> = v.try_from_type(DEFAULT_REGISTER_SIZE_KEY).optional()?;

    let rd = RegisterDescription {
        version,
        name,
        description,
        extension,
        default_register_size_in_bits,
    };

    Ok(rd)
}


#[derive(Debug)]
pub struct RegisterDescription {
    pub name: Name,
    pub description: Option<String>,
    pub version: SpecVersion,
    pub extension: Option<Extension>,
    pub default_register_size_in_bits: Option<RegisterSize>,
}

#[derive(Debug, Copy, Clone)]
pub enum SpecVersion {
    /// 0.1
    VersionZeroOne,
}

const VERSION_ZERO_ONE: &str = "0.1";

impl TryFrom<&str> for SpecVersion {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            VERSION_ZERO_ONE => Ok(SpecVersion::VersionZeroOne),
            unknown_version => Err(format!("unknown register description specification version '{}'", unknown_version))
        }
    }
}

impl fmt::Display for SpecVersion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SpecVersion::VersionZeroOne => write!(f, "{}", VERSION_ZERO_ONE)
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Extension {
    Vga,
}

const EXTENSION_VGA: &str = "vga";

impl TryFrom<&str> for Extension {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            EXTENSION_VGA => Ok(Extension::Vga),
            unknown => Err(format!("unknown extension '{}'", unknown))
        }
    }
}

impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Extension::Vga => write!(f, "{}", EXTENSION_VGA)
        }
    }
}
