use gtk::prelude::*;

pub fn build() -> gtk::Box {
    let card = super::card("Conexiones", "󰤨");
    let grid = gtk::Grid::new();
    grid.set_column_spacing(8);
    grid.set_row_spacing(8);
    let wifi = super::pill(&crate::system::wifi_label(), "󰤨", true);
    wifi.connect_clicked(|_| {
        let _ = crate::system::run_command("nodalix-wifi-menu", &[]);
    });
    grid.attach(&wifi, 0, 0, 1, 1);

    let bluetooth = super::pill(&crate::widgets::bluetooth::label(), "󰂯", true);
    bluetooth.connect_clicked(|_| {
        let _ = crate::system::run_command("nodalix-bt-menu", &[]);
    });
    grid.attach(&bluetooth, 0, 1, 1, 1);
    card.append(&grid);
    card
}
