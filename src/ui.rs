pub mod object;
pub mod editor;
pub mod field;
pub mod validate;

use crate::logic::validation::{
    ParsedFile,
    register_description::{
        RegisterDescription,
    },
};

use cursive::{
    Cursive,
    views::{
        TextView,
        Dialog,
        LinearLayout,
        SelectView,
        DummyView,
        EditView,
        Checkbox,
    },
    direction::{
        Orientation,
    },
    traits::*,
};

use self::object::{
    ObjectHandler,
};

use self::editor::open_new_register_dialog;

pub struct EditorData {
    pub register_file: ParsedFile,
    pub register_file_raw: String,
    pub file_path: String,
    pub objects: ObjectHandler,
}

impl EditorData {
    pub fn rd(&self) -> &RegisterDescription {
        &self.register_file.description
    }
}

pub fn run_ui(register_file: ParsedFile, register_file_raw: String, file_path: String) {
    let editor_data = EditorData {
        register_file,
        register_file_raw,
        file_path,
        objects: ObjectHandler::new(),
    };

    let mut c = Cursive::default();
    c.set_user_data(editor_data);
    let main_menu = create_main_menu(c.user_data().unwrap());
    c.add_layer(main_menu);
    c.run();
}

#[derive(Debug, Copy, Clone)]
pub enum MainMenu {
    AddNewRegister,
    Quit,
}


fn create_main_menu(data: &EditorData) -> Dialog {
    let l = LinearLayout::new(Orientation::Vertical)
        .child(TextView::new(&data.file_path))
        .child(DummyView)
        .child(SelectView::<MainMenu>::new()
            .item("Add new register", MainMenu::AddNewRegister)
            .item("Quit", MainMenu::Quit)
            .on_submit(main_menu_handler)
            .min_width(20));

    Dialog::new().title("Register description editor").content(l)

}

fn main_menu_handler(s: &mut Cursive, option: &MainMenu) {
    match option {
        MainMenu::AddNewRegister => {
            let data: &mut EditorData = s.user_data().unwrap();
            data.objects = ObjectHandler::new();
            drop(data);

            open_new_register_dialog(s);
        }
        MainMenu::Quit => s.quit(),
    }
}

pub fn string_from_edit_view(s: &mut Cursive, id: &'static str) -> String {
    s.call_on_id(id, |e: &mut EditView| {
        e.get_content().to_string()
    }).unwrap()
}

pub fn boolean_from_checkbox(s: &mut Cursive, id: &'static str) -> bool {
    s.call_on_id(id, |e: &mut Checkbox| {
        e.is_checked()
    }).unwrap()
}

pub fn error_message(s: &mut Cursive, message: String) {
    let d = Dialog::text(message).button("Close", |s| {
        s.pop_layer();
    });
    s.add_layer(d)
}
