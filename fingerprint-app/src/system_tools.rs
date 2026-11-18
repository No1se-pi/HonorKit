// SPDX-License-Identifier: GPL-3.0-only

use gtk::{gio, glib, prelude::*};
use std::ffi::OsStr;

const HELPER: &str = "/usr/local/libexec/honorkit-system";

pub fn append_to(root: &gtk::Box) {
    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let title = gtk::Label::new(Some("Системные компоненты"));
    title.set_xalign(0.0);
    title.add_css_class("title-3");
    root.append(&title);
    if !std::path::Path::new(HELPER).is_file() {
        let help = gtk::Label::new(Some(
            "Для установки компонентов выполните make install-system-tools в каталоге HonorKit.",
        ));
        help.set_wrap(true);
        help.set_selectable(true);
        help.set_xalign(0.0);
        root.append(&help);
        return;
    }
    add_component(root, "grub", "Тема GRUB Frieren");
    add_component(root, "audio", "Динамики Realtek ALC256");
}

fn add_component(root: &gtk::Box, component: &'static str, name: &str) {
    let label = gtk::Label::new(Some(name));
    label.set_xalign(0.0);
    root.append(&label);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let install = gtk::Button::with_label("Установить");
    let remove = gtk::Button::with_label("Удалить");
    buttons.append(&install);
    buttons.append(&remove);
    root.append(&buttons);
    let status = gtk::Label::new(None);
    status.set_xalign(0.0);
    status.set_wrap(true);
    status.set_selectable(true);
    status.add_css_class("dim-label");
    root.append(&status);

    refresh(component, &install, &remove, &status, None);

    for (button, operation) in [(&install, "install"), (&remove, "remove")] {
        let buttons = buttons.downgrade();
        let status = status.downgrade();
        let install = install.downgrade();
        let remove = remove.downgrade();
        button.connect_clicked(move |_| {
            let (Some(buttons), Some(status), Some(install), Some(remove)) = (
                buttons.upgrade(),
                status.upgrade(),
                install.upgrade(),
                remove.upgrade(),
            ) else {
                return;
            };
            buttons.set_sensitive(false);
            status.set_text("Ожидание подтверждения…");
            glib::spawn_future_local(async move {
                let message = change_component(operation, component)
                    .await
                    .err()
                    .unwrap_or_else(|| {
                        "Готово. Изменение вступит в силу при следующей загрузке.".into()
                    });
                refresh(component, &install, &remove, &status, Some(&message));
                buttons.set_sensitive(true);
            });
        });
    }
}

fn refresh(
    component: &str,
    install: &gtk::Button,
    remove: &gtk::Button,
    status: &gtk::Label,
    message: Option<&str>,
) {
    let result = std::process::Command::new(HELPER)
        .args(["status", component])
        .output()
        .ok()
        .filter(|output| output.status.success());
    let state = result.as_ref().map(|output| output.stdout.as_slice());
    install.set_sensitive(matches!(state, Some(b"absent\n" | b"repair\n")));
    remove.set_sensitive(matches!(
        state,
        Some(b"installed\n" | b"repair\n" | b"legacy\n")
    ));
    status.set_text(message.unwrap_or(match state {
        Some(b"installed\n") => "Установлен. Изменения применяются при следующей загрузке.",
        Some(b"absent\n") => "Доступен для установки.",
        Some(b"repair\n") => "Требуется переустановка или удаление темы.",
        Some(b"legacy\n") => "Установлена прежняя версия темы. Удалите её перед обновлением.",
        _ => "Не удалось определить состояние компонента.",
    }));
}

async fn change_component(operation: &str, component: &str) -> Result<(), String> {
    let args = [
        "/usr/bin/pkexec",
        "--disable-internal-agent",
        HELPER,
        operation,
        component,
    ]
    .map(OsStr::new);
    let child = gio::Subprocess::newv(&args, gio::SubprocessFlags::STDERR_PIPE)
        .map_err(|error| error.to_string())?;
    let (_, output) = child
        .communicate_utf8_future(None)
        .await
        .map_err(|error| error.to_string())?;
    if child.is_successful() {
        return Ok(());
    }
    if child.has_exited() && child.exit_status() == 126 {
        return Err("Действие отменено.".into());
    }
    let output = output
        .as_deref()
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .unwrap_or("Не удалось изменить системный компонент.");
    Err(output.into())
}
