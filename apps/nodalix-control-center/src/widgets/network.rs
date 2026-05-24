use gtk::prelude::*;

pub fn build() -> gtk::Box {
    let card = super::card("Conectividad", "󰤨");
    let grid = gtk::Grid::new();
    grid.set_column_spacing(8);
    grid.set_row_spacing(8);
    grid.attach(
        &super::pill(&crate::system::wifi_label(), "󰤨", true),
        0,
        0,
        1,
        1,
    );
    grid.attach(
        &super::pill(&crate::widgets::bluetooth::label(), "󰂯", true),
        1,
        0,
        1,
        1,
    );
    grid.attach(&super::pill("VPN", "󰖂", false), 0, 1, 1, 1);
    grid.attach(&super::pill("Nodalix Drop", "󰇚", false), 1, 1, 1, 1);
    card.append(&grid);
    card
}
