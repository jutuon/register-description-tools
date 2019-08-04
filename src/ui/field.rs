
use std::{
    convert::TryFrom,
};

use cursive::{
    Cursive,
    views::{
        Dialog,
        LinearLayout,
        EditView,
        RadioGroup,
        Checkbox,
        ViewBox,
        ListView,
    },
    view::IntoBoxedView,
    traits::*,
};

use crate::logic::validation::register::{
    AccessMode,
    RegisterSize,
    RegisterLocation,
};


pub trait Enum: TryFrom<usize, Error=String> {
    const VARIANT_NAMES: &'static [&'static str];
    fn to_index(&self) -> usize;
}

impl Enum for RegisterLocation {
    const VARIANT_NAMES: &'static [&'static str] = &["index", "rel", "abs"];
    fn to_index(&self) -> usize {
        match self {
            RegisterLocation::Index(_) => 0,
            RegisterLocation::Relative(_) => 1,
            RegisterLocation::Absolute(_) => 2,
        }
    }
}

impl Enum for AccessMode {
    const VARIANT_NAMES: &'static [&'static str] = &["r", "w", "rw"];
    fn to_index(&self) -> usize {
        *self as usize
    }
}

impl Enum for RegisterSize {
    const VARIANT_NAMES: &'static [&'static str] = &["8", "16", "32", "64"];
    fn to_index(&self) -> usize {
        match self {
            RegisterSize::Size8 => 0,
            RegisterSize::Size16 => 1,
            RegisterSize::Size32 => 2,
            RegisterSize::Size64 => 3,
        }
    }
}

/// This field doesn't set Cursive view id.
#[derive(Clone)]
pub struct EnumField<T: Enum> {
    pub key: String,
    pub value: T,
    variant_ui_order: &'static [usize],
    radio_group: Option<RadioGroup<usize>>,
}

impl <T: Enum> EnumField<T> {
    /// Panics if `variant_ui_order` is not valid.
    pub fn new<U: Into<String>>(key: U, value: T, variant_ui_order: &'static [usize]) -> Self {
        if variant_ui_order.len() != T::VARIANT_NAMES.len() {
            panic!("variant_ui_order.len() != VARIANT_NAMES.len()");
        }

        for i in variant_ui_order {
            T::VARIANT_NAMES[*i];
        }

        let key = key.into();
        Self {
            key,
            value,
            variant_ui_order,
            radio_group: None,
        }
    }
}

impl <T: Enum> TuiField for EnumField<T> {
    fn to_tui_field(&mut self) -> ViewBox {
        let mut group: RadioGroup<usize> = RadioGroup::new();
        let mut l: LinearLayout = LinearLayout::horizontal();

        for i in self.variant_ui_order {
            let mut button = group.button(*i, T::VARIANT_NAMES[*i]);
            if *i == self.value.to_index() {
                button.select();
            }
            l.add_child(button);
        }

        self.radio_group = Some(group);

        ViewBox::new(l.as_boxed_view())
    }

    fn update(&mut self, _s: &mut Cursive) {
        let enum_i = self.radio_group.as_ref().unwrap().selection().as_ref().clone();

        self.value = T::try_from(enum_i).unwrap();
    }

    fn key(&self) -> &str { &self.key }
    fn reset(&mut self) { self.value = T::try_from(self.variant_ui_order[0]).unwrap() }
}

#[derive(Clone)]
pub struct BooleanField {
    pub key: String,
    pub value: bool,
    cursive_id: String,
}

impl BooleanField {
    pub fn new<T: Into<String>>(key: T, value: bool, id_prefix: &str) -> Self {
        let key = key.into();
        Self {
            cursive_id: format!("{}{}", id_prefix, key),
            key,
            value,
        }
    }
}

impl TuiField for BooleanField {
    fn to_tui_field(&mut self) -> ViewBox {
        let view = if self.value {
            Checkbox::new().checked()
        } else {
            Checkbox::new()
        }.with_id(&self.cursive_id).as_boxed_view();
        ViewBox::new(view)
    }

    fn update(&mut self, s: &mut Cursive) {
        self.value = s.call_on_id(&self.cursive_id, |s: &mut Checkbox| s.is_checked()).unwrap();
    }

    fn key(&self) -> &str { &self.key }
    fn reset(&mut self) { self.value = false }
}

#[derive(Clone)]
pub struct StringField {
    pub key: String,
    pub value: String,
    cursive_id: String,
    validator: Option<fn(String, &str) -> Result<String, String>>,
}

impl StringField {
    pub fn new<T: Into<String>, U: Into<String>>(key: T, value: U, id_prefix: &str, validator: Option<fn(String, &str) -> Result<String, String>>) -> Self {
        let key = key.into();
        Self {
            cursive_id: format!("{}{}", id_prefix, key),
            key,
            value: value.into(),
            validator,
        }
    }
}

impl TuiField for StringField {
    fn to_tui_field(&mut self) -> ViewBox {
        let view = EditView::new().content(&self.value).with_id(&self.cursive_id).as_boxed_view();
        ViewBox::new(view)
    }

    fn update(&mut self, s: &mut Cursive) {
        self.value = s.call_on_id(&self.cursive_id, |s: &mut EditView| s.get_content().to_string()).unwrap();
    }

    fn validate(&mut self, s: &mut Cursive) -> Result<(), ()> {
        let new_value = s.call_on_id(&self.cursive_id, |s: &mut EditView| s.get_content().to_string()).unwrap();

        if let Some(validator) = &self.validator {
            error_message(s, (validator)(new_value, &self.key))?;
        }

        Ok(())
    }

    fn reset(&mut self) { self.value.clear() }
    fn key(&self) -> &str { &self.key }
}


pub trait TuiField {
    fn add_to(&mut self, l: &mut ListView) {
        l.add_child(&self.key().to_string(), self.to_tui_field())
    }
    fn validate(&mut self, _s: &mut Cursive) -> Result<(), ()> { Ok(()) }

    fn key(&self) -> &str;
    fn to_tui_field(&mut self) -> ViewBox;
    fn update(&mut self, s: &mut Cursive);
    fn reset(&mut self);
}

pub fn error_message<T>(s: &mut Cursive, result: Result<T, String>) -> Result<T, ()> {
    result.map_err(|e| {
        let d = Dialog::text(e).button("Close", |s| {
            s.pop_layer();
        });
        s.add_layer(d.title("Error"));
    })
}
