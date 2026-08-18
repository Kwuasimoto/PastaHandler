use tray_icon::TrayIconBuilder;

use crate::error::AppError;

pub struct Tray {
    pub tray_icon: tray_icon::TrayIcon,
    pub open_id: tray_icon::menu::MenuId,
    pub quit_id: tray_icon::menu::MenuId
}

impl Tray {
    pub fn new() -> Result<Self, AppError> {
        let icon = Self::load_icon_from_memory()?;

        let menu = tray_icon::menu::Menu::new();
        let open = tray_icon::menu::MenuItem::new("Open Settings", true, None);
        let quit = tray_icon::menu::MenuItem::new("Quit", true, None);
        // ids cloned before the menu takes the items — the event handler
        // compares against these
        let open_id = open.id().clone();
        let quit_id = quit.id().clone();
        menu.append_items(&[&open, &quit])?;

        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Pasta Handler")
            .with_icon(icon)
            .build()?;

        Ok(Self { tray_icon, open_id, quit_id })
    }

    fn load_icon_from_memory() -> Result<tray_icon::Icon, AppError> {
        let img = image::load_from_memory(include_bytes!("../assets/icon.png"))
            .map_err(AppError::Image)?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        tray_icon::Icon::from_rgba(rgba.into_raw(), w, h).map_err(AppError::BadIcon)
    }
}