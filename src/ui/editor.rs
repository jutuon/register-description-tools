
use std::convert::TryFrom;

use cursive::{
    Cursive,
    views::{
        Dialog,
        LinearLayout,
        ListView,
        SelectView,
        DummyView,
        Button,
        IdView,
        TextView,
    },
    traits::*,
};

use super::{
    EditorData,
    field::TuiField,
    object::{
        UiObject,
        ObjectHandler,
        UiFunction,
    },
    validate::validate_and_save_ui_register,
};

use crate::logic::validation::register::BitRange;

pub fn open_new_register_dialog(s: &mut Cursive) {
    let d = create_editor_dialog(|dialog, mut fields, select_views, buttons| {
        let (fields, select_views, add_function_button, add_enum_button) = {
            let data: &mut EditorData = s.user_data().unwrap();

            for field in data.objects.register.fields(&data.register_file) {
                field.add_to(&mut fields);
            }

            let (functions, add_function_button) = create_select_view(
                "bit field",
                &mut data.objects,
                |x| (&mut x.tmp.tmp_function, &mut x.register.functions),
                |s, i, id, getter| open_object_editor_dialog(s, i, id, getter, |_,_,_|()),
            );
            let (enums, add_enum_button) = create_select_view(
                "enum",
                &mut data.objects,
                |x| (&mut x.tmp.tmp_enum, &mut x.register.enums),
                |s, i, id, getter| {
                    open_object_editor_dialog(s, i, id, getter, |select_views, buttons, object_handler| {
                        let (enum_values, add_enum_value_button) = create_select_view(
                            "value",
                            object_handler,
                            |x| (&mut x.tmp.tmp_enum_value, &mut x.tmp.tmp_enum.values),
                            |s, i, id, getter| open_object_editor_dialog(s, i, id, getter, |_,_,_|()),
                        );

                        select_views.add_child(TextView::new("values"));
                        select_views.add_child(enum_values.scrollable());
                        buttons.add_child(add_enum_value_button);
                        buttons.add_child(DummyView);
                    });
                },
            );

            (
                fields,
                select_views.child(TextView::new("bit fields"))
                    .child(functions.scrollable())
                    .child(TextView::new("enums"))
                    .child(enums.scrollable()),
                add_function_button,
                add_enum_button
            )
        };

        let buttons = buttons.child(add_function_button)
            .child(add_enum_button)
            .child(Button::new("Fill reserved", |s| fill_empty_register_fields_as_reserved(s)))
            .child(DummyView)
            .child(Button::new("Save and next", |s| { let _ = save_register(s, true); }))
            .child(Button::new("Save", |s| { let _ = save_register(s, false); }))
            .child(Button::new("Cancel", |s| { s.pop_layer(); }));

        (dialog.title("New register"), fields, select_views, buttons)
    });

    s.add_layer(d);
}

fn save_register(mut s: &mut Cursive, next_register: bool) -> Result<(), ()> {
    modify_ui_and_data(&mut s, |mut s, editor_data| {
        let r = &mut editor_data.objects.register;

        for field in r.fields(&editor_data.register_file) {
            field.validate(&mut s)?;
        }

        for field in r.fields(&editor_data.register_file) {
            field.update(&mut s);
        }

        validate_and_save_ui_register(
            s,
            &editor_data.objects.register,
            &editor_data.register_file,
            &mut editor_data.register_file_raw,
            &editor_data.file_path,
        )?;

        Ok(())
    })?;

    if next_register {
        modify_data(&mut s, |editor_data| {
            let r = &mut editor_data.objects.register;
            r.name.reset();
            r.location.reset();
            r.description.reset();
            r.access.reset();
            r.size.reset();
            r.functions.clear();
            r.enums.clear();
            Ok(())
        })?;

        s.pop_layer();
        open_new_register_dialog(&mut s);
    } else {
        s.pop_layer();
    }

    Ok(())
}

fn fill_empty_register_fields_as_reserved(mut s: &mut Cursive) {
    let _ = modify_ui_and_data(&mut s, |mut s, editor_data| {
        let bit_fields = &mut editor_data.objects.register.functions;
        let register_size = editor_data.objects.register.size.value;
        let mut new_fields: Vec<UiFunction> = vec![];

        let mut current_msb = register_size as u16 - 1;
        let mut end_reached = false;
        for ui_field in bit_fields.iter() {
            if end_reached {
                new_fields.push(ui_field.clone());
                continue;
            }

            let range = super::field::error_message(&mut s, BitRange::try_from(ui_field.bit.value.as_str().trim()))?;

            if current_msb > range.msb {
                let new_range = BitRange::new(current_msb, range.msb + 1);
                let new_ui_field = UiFunction::new_reserved(&new_range.to_string());
                new_fields.push(new_ui_field);
            } else if current_msb < range.msb {
                end_reached = true;
            }

            new_fields.push(ui_field.clone());

            if range.lsb == 0 {
                end_reached = true;
            } else {
                current_msb = range.lsb - 1;
            }
        }

        if !end_reached {
            let new_range = BitRange::new(current_msb, 0);
            let new_ui_field = UiFunction::new_reserved(&new_range.to_string());
            new_fields.push(new_ui_field);
        }

        *bit_fields = new_fields;

        update_select_view(&mut s, "bit field", &bit_fields);

        Ok(())
    });
}

fn create_editor_dialog<T: FnMut(Dialog, ListView, LinearLayout, LinearLayout) -> (Dialog, ListView, LinearLayout, LinearLayout)>(mut set_widgets: T) -> Dialog {
    let d = Dialog::new();
    let fields = ListView::new();
    let select_views = LinearLayout::vertical();
    let buttons = LinearLayout::vertical();

    let (d, fields, select_views, buttons) = (set_widgets)(d, fields, select_views, buttons);

    let left_side = LinearLayout::vertical()
        .child(fields)
        .child(select_views);

    let l = LinearLayout::horizontal()
        .child(left_side.min_height(15).min_width(40))
        .child(DummyView)
        .child(buttons.min_width(10));

    d.content(l)
}

fn create_select_view<
    T: 'static + ToString + UiObject + Clone + Default,
    U: Fn(&mut ObjectHandler) -> (&mut T, &mut Vec<T>) + 'static + Copy,
    V: Fn(&mut Cursive, Option<usize>, &'static str, U) + 'static + Copy
>(
    select_view_id: &'static str,
    objects: &mut ObjectHandler,
    tmp_and_data_getter: U,
    edit_function: V,
) -> (IdView<SelectView<usize>>, Button) {
    let (_, data) = (tmp_and_data_getter)(objects);

    let mut objects = SelectView::<usize>::new();
    for (i, o) in data.iter_mut().enumerate() {
        objects.add_item(o.to_string(), i);
    }
    let objects = objects.on_submit(move |s, i| {
        (edit_function)(s, Some(*i), select_view_id, tmp_and_data_getter);
    }).with_id(select_view_id);

    let add_button = Button::new(format!("Add {}", select_view_id), move |s| (edit_function)(s, None, select_view_id, tmp_and_data_getter));

    (objects, add_button)
}

fn open_object_editor_dialog<
    T: 'static + ToString + UiObject + Clone + Default,
    U: Fn(&mut LinearLayout, &mut LinearLayout, &mut ObjectHandler)
>(
    s: &mut Cursive,
    object_i: Option<usize>,
    select_view_id: &'static str,
    tmp_and_data_getter: fn(&mut ObjectHandler) -> (&mut T, &mut Vec<T>),
    add_select_views: U,
) {
    let d = create_editor_dialog(|mut dialog, mut fields, mut select_views, mut buttons| {
        let editor_data: &mut EditorData = s.user_data().unwrap();
        let (tmp, data) = (tmp_and_data_getter)(&mut editor_data.objects);

        if let Some(i) = object_i {
            *tmp = data[i].clone();
        } else {
            *tmp = T::default();
        }

        for field in tmp.fields(&editor_data.register_file) {
            field.add_to(&mut fields);
        }

        (add_select_views)(&mut select_views, &mut buttons, &mut editor_data.objects);

        buttons.add_child(Button::new("Save", move |s| { let _ = save_object(s, object_i.clone(), select_view_id, tmp_and_data_getter); }));

        if let Some(function_i) = object_i.clone() {
            buttons.add_child(Button::new("Delete", move |s| delete_object(s, function_i, select_view_id, tmp_and_data_getter)));
        }

        buttons.add_child(Button::new("Cancel", |s| { s.pop_layer(); }));

        if object_i.is_some() {
            dialog.set_title(format!("Edit {}", select_view_id));
        } else {
            dialog.set_title(format!("New {}", select_view_id));
        }

        (dialog, fields, select_views, buttons)
    });

    s.add_layer(d)
}

fn save_object<
    T: ToString + UiObject + Clone,
    U: Fn(&mut ObjectHandler) -> (&mut T, &mut Vec<T>),
>(mut s: &mut Cursive, object_i: Option<usize>, select_view_id: &str, tmp_and_data_getter: U) -> Result<(), ()> {
    modify_ui_and_data(&mut s, |mut s, editor_data| {
        let (tmp, data) = (tmp_and_data_getter)(&mut editor_data.objects);
        for field in tmp.fields(&editor_data.register_file) {
            field.validate(&mut s)?;
        }

        for field in tmp.fields(&editor_data.register_file) {
            field.update(&mut s);
        }

        if let Some(object_i) = object_i {
            data[object_i] = tmp.clone();
        } else {
            data.push(tmp.clone())
        }

        update_select_view(&mut s, select_view_id, &data);

        Ok(())
    })?;

    s.pop_layer();
    Ok(())
}

fn delete_object<
    T: ToString,
    U: Fn(&mut ObjectHandler) -> (&mut T, &mut Vec<T>),
>(mut s: &mut Cursive, object_i: usize, select_view_id: &str, tmp_and_data_getter: U) {
    let _ = modify_ui_and_data(&mut s, |mut s, data| {
        let (_, data) = (tmp_and_data_getter)(&mut data.objects);
        data.remove(object_i);
        update_select_view(&mut s, select_view_id, &data);
        Ok(())
    });
    s.pop_layer();
}

fn modify_ui_and_data<T: FnMut(&mut Cursive, &mut EditorData) -> Result<(),()>>(mut s: &mut Cursive, mut function: T) -> Result<(), ()> {
    let mut data: EditorData = s.take_user_data().unwrap();
    let r = (function)(&mut s, &mut data);
    s.set_user_data(data);
    r
}

fn modify_data<T: FnMut(&mut EditorData) -> Result<(),()>>(s: &mut Cursive, mut function: T) -> Result<(), ()> {
    let mut data: EditorData = s.take_user_data().unwrap();
    let r = (function)(&mut data);
    s.set_user_data(data);
    r
}

fn update_select_view<T: ToString>(s: &mut Cursive, select_view_id: &str, data: &Vec<T>) {
    s.call_on_id(select_view_id, |v: &mut SelectView<usize>| {
        v.clear();
        for (i, f) in data.iter().enumerate() {
            v.add_item(f.to_string(), i);
        }
    }).unwrap();
}
