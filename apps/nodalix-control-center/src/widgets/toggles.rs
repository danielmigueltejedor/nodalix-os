use gtk::prelude::*;

pub fn build() -> gtk::Box {
    let card = super::card("Modos", "󰔎");
    let grid = gtk::Grid::new();
    grid.set_column_spacing(8);
    grid.set_row_spacing(8);
    grid.attach(&super::pill("No molestar", "󰂛", false), 0, 0, 1, 1);
    grid.attach(&super::pill("Noche", "󰖔", false), 0, 1, 1, 1);
    grid.attach(&super::pill("Oscuro", "󰔎", true), 0, 2, 1, 1);
    card.append(&grid);
    card
}
