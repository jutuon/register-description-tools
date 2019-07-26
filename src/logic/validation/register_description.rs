

use std::{
    convert::TryFrom,
    fmt,
};

use super::{ValidationError, CurrentTable, ErrorContext, TableValidator, TomlTable};

const VERSION_KEY: &str = "version";
const NAME_KEY: &str = "name";
const DESCRIPTION_KEY: &str = "description";
const DEFAULT_REGISTER_SIZE_KEY: &str = "default_register_size_in_bits";
const EXTENSION_KEY: &str = "extension";

const POSSIBLE_KEYS: &[&str] = &[VERSION_KEY, NAME_KEY, DESCRIPTION_KEY, DEFAULT_REGISTER_SIZE_KEY, EXTENSION_KEY];

pub fn check_register_description(table: &TomlTable, errors: &mut Vec<ValidationError>) -> Result<RegisterDescription, ()> {
    let mut ec = ErrorContext::new(CurrentTable::RegisterDescription, VERSION_KEY, errors);
    super::check_unknown_keys(table, POSSIBLE_KEYS, &mut ec);

    let mut v = TableValidator::new(table, &mut ec);

    let version: SpecVersion = v.try_from_type(VERSION_KEY).require()?;
    let name = v.string(NAME_KEY).require()?;

    let description = v.string(DESCRIPTION_KEY).optional()?;
    let extension: Option<Extension> = v.try_from_type(EXTENSION_KEY).optional()?;
    let default_register_size_in_bits = v.u16(DEFAULT_REGISTER_SIZE_KEY).optional()?;

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
    pub name: String,
    pub description: Option<String>,
    pub version: SpecVersion,
    pub extension: Option<Extension>,
    pub default_register_size_in_bits: Option<u16>,
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
            unknown_version => Err(format!("Unknown register description specification version '{}'", unknown_version))
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
            unknown => Err(format!("Unknown extension '{}'", unknown))
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
