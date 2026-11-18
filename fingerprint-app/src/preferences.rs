use crate::unlock::{self, Preferences};
use gtk::{gio, glib, prelude::*};
use std::{cell::Cell, rc::Rc};

const TOOL: &str = "/usr/local/libexec/honorkit-touchpad-config";
const GESTURE_CONFIG: &str = "/etc/honorkit-gestures.conf";
const DEFAULT: u32 = 31;
const OPTIONS: &[&str] = &[
    "Яркость",
    "Громкость",
    "Снимки экрана",
    "Свернуть и закрыть окно",
    "Открыть уведомления",
    "Поменять местами края яркости и громкости",
];

async fn change_touchpad(action: &str, value: u32) -> Result<(), String> {
    let value = value.to_string();
    let args: Vec<_> = ["/usr/bin/pkexec", TOOL, action, &value]
        .into_iter()
        .map(std::ffi::OsStr::new)
        .collect();
    let child = gio::Subprocess::newv(&args, gio::SubprocessFlags::STDERR_PIPE)
        .map_err(|e| e.to_string())?;
    let (_, error) = child
        .communicate_utf8_future(None)
        .await
        .map_err(|e| e.to_string())?;
    if child.is_successful() {
        Ok(())
    } else if child.has_exited() && child.exit_status() == 126 {
        Err("Действие отменено.".into())
    } else {
        if let Some(error) = error {
            eprintln!("HonorKit touchpad: {error}");
        }
        Err(
            "Не удалось изменить настройку. Проверьте доступ к тачпаду и пароль администратора."
                .into(),
        )
    }
}

fn label(text: &str) -> gtk::Label {
    let item = gtk::Label::new(Some(text));
    item.set_wrap(true);
    item.set_xalign(0.0);
    item
}

fn heading(text: &str) -> gtk::Label {
    let item = label(text);
    item.add_css_class("title-3");
    item
}

pub fn show(parent: &gtk::ApplicationWindow) {
    let window = gtk::Window::builder()
        .title("Настройки HonorKit")
        .transient_for(parent)
        .modal(true)
        .default_width(525)
        .default_height(640)
        .build();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 12);
    root.set_margin_top(20);
    root.set_margin_bottom(24);
    root.set_margin_start(24);
    root.set_margin_end(24);
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&root)
        .build();
    window.set_child(Some(&scroll));

    let prefs = Preferences::load();
    root.append(&heading("Анимация разблокировки"));
    let enabled = gtk::CheckButton::with_label("Показывать после разблокировки отпечатком");
    enabled.set_active(prefs.enabled);
    root.append(&enabled);
    let style = gtk::DropDown::from_strings(&["Зелёный отпечаток", "Световой проход"]);
    style.set_selected(u32::from(prefs.scan));
    style.set_tooltip_text(Some("Стиль анимации"));
    let duration = gtk::DropDown::from_strings(&["Обычная · 1 секунда", "Длинная · 1,6 секунды"]);
    duration.set_selected(u32::from(prefs.long));
    duration.set_tooltip_text(Some("Длительность анимации"));
    let choices = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    choices.append(&style);
    choices.append(&duration);
    root.append(&choices);
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let save = gtk::Button::with_label("Сохранить анимацию");
    save.add_css_class("suggested-action");
    let preview = gtk::Button::with_label("Предпросмотр");
    actions.append(&save);
    actions.append(&preview);
    root.append(&actions);
    let animation_status = label("");
    animation_status.add_css_class("dim-label");
    root.append(&animation_status);

    let values = move || Preferences {
        enabled: enabled.is_active(),
        scan: style.selected() == 1,
        long: duration.selected() == 1,
    };
    let saved_values = values.clone();
    save.connect_clicked(move |_| match saved_values().save() {
        Ok(()) => animation_status.set_text("Настройки анимации сохранены."),
        Err(error) => animation_status.set_text(&format!("Ошибка сохранения: {error}")),
    });
    let weak_parent = parent.downgrade();
    let weak_window = window.downgrade();
    preview.connect_clicked(move |_| {
        if let Some(app) = weak_parent
            .upgrade()
            .and_then(|parent| parent.application())
        {
            unlock::flash(&app, values(), weak_window.upgrade().as_ref());
        }
    });

    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    root.append(&heading("Жесты тачпада"));
    let supported = std::path::Path::new(TOOL).is_file();
    let mask = std::fs::read_to_string(GESTURE_CONFIG)
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok())
        .filter(|mask| *mask <= 63)
        .unwrap_or(DEFAULT);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 3);
    let mut checks = Vec::new();
    for (index, name) in OPTIONS.iter().enumerate() {
        let check = gtk::CheckButton::with_label(name);
        check.set_active(mask & (1 << index) != 0);
        list.append(&check);
        checks.push(check);
    }
    list.set_sensitive(supported);
    root.append(&list);
    let apply = gtk::Button::with_label("Сохранить жесты");
    apply.set_halign(gtk::Align::Start);
    apply.set_sensitive(supported);
    root.append(&apply);

    root.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    root.append(&heading("Тактильный отклик"));
    let info = label("Уровень отправляется в прошивку. Текущее значение тачпад не сообщает.");
    info.add_css_class("dim-label");
    root.append(&info);
    let level = gtk::DropDown::from_strings(&["Выбрать уровень", "Слабый", "Средний", "Сильный"]);
    level.set_sensitive(supported);
    let send = gtk::Button::with_label("Установить силу");
    send.set_sensitive(false);
    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    controls.append(&level);
    controls.append(&send);
    root.append(&controls);
    let status = label(if supported {
        ""
    } else {
        "Системный модуль настроек пока не установлен. Управление отпечатками и анимация доступны."
    });
    status.add_css_class("dim-label");
    root.append(&status);

    let haptics_busy = Rc::new(Cell::new(false));
    let update = send.clone();
    let busy = haptics_busy.clone();
    level.connect_selected_notify(move |item| {
        update.set_sensitive(supported && !busy.get() && matches!(item.selected(), 1..=3))
    });
    let gesture_status = status.clone();
    apply.connect_clicked(move |button| {
        let mask = checks.iter().enumerate().fold(0, |mask, (index, check)| {
            mask | (u32::from(check.is_active()) << index)
        });
        apply_with_feedback(
            button,
            &gesture_status,
            "gestures",
            mask,
            "Настройки жестов сохранены.",
            || {},
        );
    });
    send.connect_clicked(move |button| {
        if let 1..=3 = level.selected()
            && !haptics_busy.replace(true)
        {
            let level = level.clone();
            level.set_sensitive(false);
            let busy = haptics_busy.clone();
            apply_with_feedback(
                button,
                &status,
                "haptics",
                level.selected() - 1,
                "Команда отправлена. Проверьте отклик касанием.",
                move || {
                    busy.set(false);
                    level.set_sensitive(true);
                },
            );
        }
    });
    crate::system_tools::append_to(&root);
    window.present();
}

fn apply_with_feedback(
    button: &gtk::Button,
    status: &gtk::Label,
    action: &'static str,
    value: u32,
    success: &'static str,
    finished: impl FnOnce() + 'static,
) {
    button.set_sensitive(false);
    status.set_text("Ожидание подтверждения…");
    let button = button.clone();
    let status = status.clone();
    glib::spawn_future_local(async move {
        let result = change_touchpad(action, value).await;
        match result {
            Ok(()) => status.set_text(success),
            Err(message) => status.set_text(&message),
        }
        finished();
        button.set_sensitive(true);
    });
}
