use tray_icon::TrayIconBuilder;

use crate::error::AppError;

pub struct Tray {
    pub tray_icon: tray_icon::TrayIcon,
    pub open_id: tray_icon::menu::MenuId,
    pub quit_id: tray_icon::menu::MenuId
}

impl Tray {
    pub fn new() -> Result<Self, AppError> {
        // Load icon
        let icon = Self::load_icon_from_memory()?;

        // Create menu and items objects
        let menu = tray_icon::menu::Menu::new();
        let open = tray_icon::menu::MenuItem::new("Open Settings", true, None);
        let quit = tray_icon::menu::MenuItem::new("Quit", true, None);

        // Clone ids
        let open_id = open.id().clone();
        let quit_id = quit.id().clone();

        // Assign menu items to menu
        menu.append_items(&[&open, &quit])?;
        
        // Build a tray icon
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("PastaHandler")
            .with_icon(icon)
            .build()?;

        Ok(Self { tray_icon, open_id, quit_id })
    }

    pub fn load_icon_from_memory() -> Result<tray_icon::Icon, AppError> {
        let img = image::load_from_memory(include_bytes!("../assets/icon.png"))
            .map_err(AppError::Image)?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let icon = tray_icon::Icon::from_rgba(rgba.into_raw(), w,  h)
            .map_err(AppError::BadIcon)?;
        Ok(icon)
    }
}