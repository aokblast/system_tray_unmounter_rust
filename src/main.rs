use std::{path, time::{Duration, Instant}};
use sysinfo::{Disk, DiskExt, System, RefreshKind, SystemExt};
use tao::{menu::{self, MenuItemAttributes, CustomMenuItem, MenuType}, system_tray::{self, SystemTrayBuilder}, TrayId, event_loop::{EventLoop, ControlFlow}, event::{Event, StartCause}};
use image::io::Reader as ImageReader;
use byte_unit::Byte;
use daemonize::Daemonize;

fn main() {
    Daemonize::new().start().unwrap();

    let icon_path = "/Applications/System-Tray-Mounter.app/Contents/Resources/icon.png";
    let icon = load_icon(path::Path::new(icon_path));
    let tray_id = TrayId::new("Mountd");
    let event_loop = EventLoop::new();

    let mut sysinfo = System::new_with_specifics(RefreshKind::new().with_disks().with_disks_list());

    let mut tray_menu = None;

    let (mut disk_items, mut quit_item) = refresh_tray_menu(&mut tray_menu, sysinfo.disks());

    let system_tray = SystemTrayBuilder::new(icon, None)
        .with_id(tray_id)
        .with_tooltip("System Tray Mounter - Show all mounted disk part")
        .build(&event_loop).unwrap();

    let mut system_tray = Some(system_tray);

    event_loop.run(move |event, _event_loop, control_flow| {
        *control_flow = ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(500));

        match event {
            Event::MenuEvent { window_id: _window_id , menu_id, origin: MenuType::ContextMenu, .. } => {
                if menu_id == quit_item.clone().id() {
                    system_tray.take();
                    *control_flow = ControlFlow::Exit;
                } else {
                    for idx in 0..disk_items.len() {
                        if disk_items.get(idx).unwrap().clone().id() == menu_id && sysinfo.disks().get(idx + 1).unwrap().is_removable() {
                            std::process::Command::new("diskutil").arg("unmount").arg(sysinfo.disks().get(idx + 1).unwrap().mount_point()).output().unwrap();
                            break;
                        }
                    }
                }
            }

            Event::NewEvents(StartCause::Init) | Event::NewEvents(StartCause::ResumeTimeReached{..}) => {
                sysinfo.refresh_disks();
                sysinfo.refresh_disks_list();
                (disk_items, quit_item) = refresh_tray_menu(&mut tray_menu, sysinfo.disks());
                system_tray.as_mut().unwrap().set_menu(tray_menu.as_ref().unwrap());
            }

            _ => {

            }
        }
    });

}


fn load_icon(path: &std::path::Path) -> system_tray::Icon {
    let (icon_rgba, icon_width, icon_height) = {
        let img = ImageReader::open(path).expect("Cannot find file").decode().unwrap();
        let rgba = img.as_rgba8().unwrap().as_raw().to_vec();
        (rgba, img.width(), img.height())
    };

    system_tray::Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap()
}

fn refresh_tray_menu(tray_menu: &mut Option<menu::ContextMenu>, disks: &[Disk]) -> (Vec<CustomMenuItem>, CustomMenuItem) {
    let mut menu = menu::ContextMenu::new();
    let mut disk_items = vec![];

    for idx in 1..disks.len() {
        let disk = &disks.get(idx).unwrap();
        let disk_format = &format!("{}: {}/{}", disk.name().to_str().unwrap()
                                   , Byte::from_bytes(disk.available_space().into()).get_appropriate_unit(true).to_string()
                                   , Byte::from_bytes(disk.total_space().into()).get_appropriate_unit(true).to_string());
        disk_items.push(menu.add_item(MenuItemAttributes::new(&disk_format)));
    }

    let quit = menu.add_item(MenuItemAttributes::new("Quit"));

    tray_menu.replace(menu);

    (disk_items, quit)
}
