use gtk::prelude::*;

pub fn build() -> gtk::Box {
    let card = super::card("Acciones", "󰒓");
    let grid = gtk::Grid::new();
    grid.set_column_spacing(8);
    grid.set_row_spacing(8);
    for (index, (label, icon, command)) in [
        ("Captura", "󰄀", "grim"),
        ("Ajustes", "󰒓", "nodalix-settings"),
        ("Archivos", "󰉋", "nodalix-files"),
    ]
    .iter()
    .enumerate()
    {
        let button = super::pill(label, icon, false);
        let command = (*command).to_string();
        button.connect_clicked(move |_| {
            if crate::system::command_exists(&command) {
                let _ = crate::system::run_command(&command, &[]);
            }
        });
        grid.attach(&button, (index % 2) as i32, (index / 2) as i32, 1, 1);
    }
    card.append(&grid);
    card
}
