mod art;
mod fprint;

use art::{Art, Mode};
use fprint::{Reader, error_text};
use gtk::{gio, glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

const FINGERS: &[(&str, &str)] = &[
    ("left-thumb", "Большой · левая рука"),
    ("left-index-finger", "Указательный · левая рука"),
    ("left-middle-finger", "Средний · левая рука"),
    ("left-ring-finger", "Безымянный · левая рука"),
    ("left-little-finger", "Мизинец · левая рука"),
    ("right-thumb", "Большой · правая рука"),
    ("right-index-finger", "Указательный · правая рука"),
    ("right-middle-finger", "Средний · правая рука"),
    ("right-ring-finger", "Безымянный · правая рука"),
    ("right-little-finger", "Мизинец · правая рука"),
];

struct Enrollment {
    reader: Reader,
    _statuses: gio::SignalSubscription,
    owner_watch: glib::SignalHandlerId,
    started: bool,
    pending_result: Option<(Mode, String)>,
    stages: u32,
    passed: u32,
}

struct View {
    window: gtk::ApplicationWindow,
    reader_name: gtk::Label,
    art: Art,
    status: gtk::Label,
    progress: gtk::ProgressBar,
    list: gtk::ListBox,
    choices: gtk::DropDown,
    choice_names: gtk::StringList,
    add: gtk::Button,
    cancel: gtk::Button,
    reload: gtk::Button,
    registered: RefCell<Vec<String>>,
    available: RefCell<Vec<&'static str>>,
    busy: Cell<bool>,
    ready: Cell<bool>,
    closing: Cell<bool>,
    enrollment: RefCell<Option<Enrollment>>,
}

impl View {
    fn new(app: &gtk::Application) -> Rc<Self> {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("HonorKit")
            .default_width(510)
            .default_height(490)
            .build();
        let header = gtk::HeaderBar::new();
        let reload = gtk::Button::from_icon_name("view-refresh-symbolic");
        reload.set_tooltip_text(Some("Обновить список"));
        header.pack_start(&reload);
        window.set_titlebar(Some(&header));
        let content = gtk::Box::new(gtk::Orientation::Vertical, 20);
        content.set_margin_top(20);
        content.set_margin_bottom(24);
        content.set_margin_start(24);
        content.set_margin_end(24);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&content)
            .build();
        window.set_child(Some(&scroll));

        let hero = gtk::Box::new(gtk::Orientation::Vertical, 7);
        hero.add_css_class("scanner");
        let art = Art::new();
        hero.append(&art.widget);
        let status = gtk::Label::new(Some("Поиск сканера…"));
        status.add_css_class("scanner-status");
        status.set_wrap(true);
        status.set_justify(gtk::Justification::Center);
        hero.append(&status);
        let reader_name = gtk::Label::new(None);
        reader_name.add_css_class("scanner-reader");
        reader_name.set_wrap(true);
        hero.append(&reader_name);
        let progress = gtk::ProgressBar::builder()
            .show_text(true)
            .visible(false)
            .build();
        progress.set_margin_start(24);
        progress.set_margin_end(24);
        hero.append(&progress);
        content.append(&hero);

        let saved = gtk::Box::new(gtk::Orientation::Vertical, 10);
        let title = gtk::Label::new(Some("Сохранённые отпечатки"));
        title.set_xalign(0.0);
        title.add_css_class("section-title");
        saved.append(&title);
        let list = gtk::ListBox::new();
        list.add_css_class("boxed-prints");
        list.set_selection_mode(gtk::SelectionMode::None);
        saved.append(&list);
        content.append(&saved);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let choice_names = gtk::StringList::new(&[]);
        let choices = gtk::DropDown::builder()
            .model(&choice_names)
            .hexpand(true)
            .build();
        choices.set_tooltip_text(Some("Выберите палец для регистрации"));
        let add = gtk::Button::with_label("Добавить");
        add.add_css_class("suggested-action");
        let cancel = gtk::Button::with_label("Отменить регистрацию");
        cancel.set_visible(false);
        actions.append(&choices);
        actions.append(&add);
        actions.append(&cancel);
        content.append(&actions);

        let view = Rc::new(Self {
            window,
            reader_name,
            art,
            status,
            progress,
            list,
            choices,
            choice_names,
            add,
            cancel,
            reload,
            registered: RefCell::new(Vec::new()),
            available: RefCell::new(Vec::new()),
            busy: Cell::new(false),
            ready: Cell::new(false),
            closing: Cell::new(false),
            enrollment: RefCell::new(None),
        });
        view.connect();
        view.window.present();
        view.refresh(None);
        view
    }

    fn connect(self: &Rc<Self>) {
        let view = Rc::downgrade(self);
        self.reload.connect_clicked(move |_| {
            if let Some(view) = view.upgrade() {
                view.refresh(None);
            }
        });
        let view = Rc::downgrade(self);
        self.add.connect_clicked(move |_| {
            if let Some(view) = view.upgrade() {
                let finger = view
                    .available
                    .borrow()
                    .get(view.choices.selected() as usize)
                    .copied();
                if let Some(finger) = finger {
                    view.enroll(finger);
                }
            }
        });
        let view = Rc::downgrade(self);
        self.cancel.connect_clicked(move |_| {
            if let Some(view) = view.upgrade() {
                view.end_enrollment(Mode::Idle, "Регистрация отменена.".into());
            }
        });
        // The window retains its controller until GTK destroys the window.
        let view = self.clone();
        self.window.connect_close_request(move |_| {
            if !view.busy.get() {
                return glib::Propagation::Proceed;
            }
            view.closing.set(true);
            let started = view
                .enrollment
                .borrow()
                .as_ref()
                .is_some_and(|item| item.started);
            if started {
                view.end_enrollment(Mode::Idle, "Регистрация отменена.".into());
            }
            glib::Propagation::Stop
        });
    }

    fn show(&self, mode: Mode, message: &str) {
        self.art.set(mode);
        self.status.set_text(message);
    }

    fn set_busy(&self, busy: bool) {
        self.busy.set(busy);
        self.reload.set_sensitive(!busy);
        self.list.set_sensitive(!busy);
        let can_add = !busy && self.ready.get() && !self.available.borrow().is_empty();
        self.add.set_sensitive(can_add);
        self.choices.set_sensitive(can_add);
    }

    fn refresh(self: &Rc<Self>, result: Option<(Mode, String)>) {
        if self.busy.get() {
            return;
        }
        self.set_busy(true);
        if result.is_none() {
            self.show(Mode::Idle, "Поиск сканера…");
        }
        let view = self.clone();
        glib::spawn_future_local(async move {
            let found = async {
                let reader = Reader::connect().await?;
                let fingers = reader.list().await?;
                Ok::<_, glib::Error>((reader, fingers))
            }
            .await;
            match found {
                Ok((reader, fingers)) => {
                    view.reader_name.set_text(&reader.name());
                    view.ready.set(true);
                    view.render_list(fingers);
                    if let Some((mode, message)) = result {
                        view.show(mode, &message);
                    } else {
                        view.show(Mode::Idle, "Сканер готов");
                    }
                }
                Err(error) => {
                    view.reader_name.set_text("");
                    view.ready.set(false);
                    view.render_list(Vec::new());
                    let message = error_text(&error);
                    if let Some((_, previous)) = result {
                        view.show(Mode::Error, &format!("{previous}\n{message}"));
                    } else {
                        view.show(Mode::Error, &message);
                    }
                }
            }
            view.set_busy(false);
            if view.closing.get() {
                view.window.close();
            }
        });
    }

    fn render_list(self: &Rc<Self>, mut fingers: Vec<String>) {
        fingers.sort_by_key(|finger| {
            FINGERS
                .iter()
                .position(|(id, _)| id == finger)
                .unwrap_or(FINGERS.len())
        });
        *self.registered.borrow_mut() = fingers;
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let fingers = self.registered.borrow();
        for finger in fingers.iter() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 9);
            row.add_css_class("print-row");
            row.append(&gtk::Image::from_icon_name("security-high-symbolic"));
            let label = gtk::Label::new(Some(finger_label(finger)));
            label.set_hexpand(true);
            label.set_xalign(0.0);
            label.set_wrap(true);
            row.append(&label);
            let remove = gtk::Button::with_label("Удалить");
            remove.add_css_class("flat");
            row.append(&remove);
            let view = Rc::downgrade(self);
            let id = finger.clone();
            remove.connect_clicked(move |_| {
                if let Some(view) = view.upgrade() {
                    view.confirm_delete(id.clone());
                }
            });
            self.list.append(&row);
        }
        if fingers.is_empty() {
            let empty = gtk::Label::new(Some(if self.ready.get() {
                "Пока нет сохранённых отпечатков"
            } else {
                "Список недоступен"
            }));
            empty.add_css_class("print-row");
            empty.add_css_class("dim-label");
            self.list.append(&empty);
        }
        let available: Vec<_> = FINGERS
            .iter()
            .filter(|(id, _)| self.ready.get() && !fingers.iter().any(|finger| finger == id))
            .collect();
        let labels: Vec<_> = available.iter().map(|(_, label)| *label).collect();
        *self.available.borrow_mut() = available.iter().map(|(id, _)| *id).collect();
        self.choice_names
            .splice(0, self.choice_names.n_items(), &labels);
        self.choices.set_selected(if labels.is_empty() {
            gtk::INVALID_LIST_POSITION
        } else {
            0
        });
    }

    fn complete(self: &Rc<Self>, mode: Mode, message: String) {
        self.cancel.set_visible(false);
        self.add.set_visible(true);
        self.choices.set_visible(true);
        self.progress.set_visible(false);
        self.set_busy(false);
        if self.closing.get() {
            self.window.close();
        } else {
            self.refresh(Some((mode, message)));
        }
    }

    fn confirm_delete(self: &Rc<Self>, finger: String) {
        if self.busy.get() {
            return;
        }
        let dialog = gtk::AlertDialog::builder()
            .modal(true)
            .message("Удалить отпечаток?")
            .detail(format!(
                "{} больше нельзя будет использовать для входа.",
                finger_label(&finger)
            ))
            .build();
        dialog.set_buttons(&["Отмена", "Удалить отпечаток"]);
        dialog.set_cancel_button(0);
        dialog.set_default_button(0);
        let view = self.clone();
        glib::spawn_future_local(async move {
            if dialog.choose_future(Some(&view.window)).await == Ok(1) {
                view.delete(finger);
            }
        });
    }

    fn delete(self: &Rc<Self>, finger: String) {
        if self.busy.get() {
            return;
        }
        self.set_busy(true);
        self.show(Mode::Idle, "Удаление отпечатка…");
        let view = self.clone();
        glib::spawn_future_local(async move {
            let outcome = async {
                let reader = Reader::connect().await?;
                reader.call("Claim", Some("")).await?;
                let deleted = reader.call("DeleteEnrolledFinger", Some(&finger)).await;
                let released = reader.call("Release", None).await;
                deleted.and(released)
            }
            .await;
            match outcome {
                Ok(_) => view.complete(Mode::Passed, "Отпечаток удалён".into()),
                Err(error) => view.complete(Mode::Error, error_text(&error)),
            }
        });
    }

    fn enroll(self: &Rc<Self>, finger: &'static str) {
        if self.busy.get() {
            return;
        }
        self.set_busy(true);
        self.show(Mode::Idle, "Ожидание доступа к сканеру…");
        let view = self.clone();
        glib::spawn_future_local(async move {
            let reader = match Reader::connect().await {
                Ok(reader) => reader,
                Err(error) => return view.complete(Mode::Error, error_text(&error)),
            };
            if let Err(error) = reader.call("Claim", Some("")).await {
                return view.complete(Mode::Error, error_text(&error));
            }
            let stop = match reader.list().await {
                Ok(_) if view.closing.get() => Some((Mode::Idle, "Регистрация отменена.".into())),
                Ok(fingers) if fingers.iter().any(|saved| saved == finger) => {
                    Some((Mode::Error, "Этот палец уже зарегистрирован.".into()))
                }
                Ok(_) => None,
                Err(error) => Some((Mode::Error, error_text(&error))),
            };
            if let Some((mode, message)) = stop {
                let _ = reader.call("Release", None).await;
                return view.complete(mode, message);
            }
            let weak = Rc::downgrade(&view);
            let statuses = reader.watch_enrollment(move |message, done| {
                if let Some(view) = weak.upgrade() {
                    view.enrollment_status(&message, done);
                }
            });
            let weak = Rc::downgrade(&view);
            let owner_watch = reader.watch_owner(move || {
                if let Some(view) = weak.upgrade() {
                    view.end_enrollment(Mode::Error, "Соединение со сканером потеряно.".into());
                }
            });
            *view.enrollment.borrow_mut() = Some(Enrollment {
                stages: reader.stages(),
                reader: reader.clone(),
                _statuses: statuses,
                owner_watch,
                started: false,
                pending_result: None,
                passed: 0,
            });
            view.show(Mode::Scan, "Приложите палец к сканеру");
            view.progress.set_visible(true);
            view.update_progress(0, reader.stages());
            match reader.call("EnrollStart", Some(finger)).await {
                Ok(_) => {
                    let pending = {
                        let mut enrollment = view.enrollment.borrow_mut();
                        let Some(active) = enrollment.as_mut() else {
                            return;
                        };
                        active.started = true;
                        active.pending_result.take()
                    };
                    if let Some((mode, message)) = pending {
                        view.end_enrollment(mode, message);
                    } else if view.closing.get() {
                        view.end_enrollment(Mode::Idle, "Регистрация отменена.".into());
                    } else {
                        view.add.set_visible(false);
                        view.choices.set_visible(false);
                        view.cancel.set_visible(true);
                    }
                }
                Err(error) => {
                    if let Some(active) = view.enrollment.borrow_mut().as_mut() {
                        active.started = true; // Best-effort stop also covers a timed-out D-Bus reply.
                    }
                    view.end_enrollment(Mode::Error, error_text(&error));
                }
            }
        });
    }

    fn update_progress(&self, passed: u32, stages: u32) {
        if stages == 0 {
            self.progress.pulse();
            self.progress
                .set_text(Some(&format!("Принято касаний: {passed}")));
        } else {
            self.progress
                .set_fraction(f64::from(passed.min(stages)) / f64::from(stages));
            self.progress
                .set_text(Some(&format!("{passed} / {stages} касаний")));
        }
    }

    fn enrollment_status(self: &Rc<Self>, status: &str, done: bool) {
        let (mode, message) = match status {
            "enroll-completed" => (Mode::Passed, "Отпечаток сохранён"),
            "enroll-stage-passed" => {
                if let Some(active) = self.enrollment.borrow_mut().as_mut() {
                    active.passed += 1;
                    self.update_progress(active.passed, active.stages);
                }
                (
                    Mode::Scan,
                    "Уберите палец и приложите снова под другим углом",
                )
            }
            "enroll-remove-and-retry" => (Mode::Lift, "Полностью уберите палец, затем повторите"),
            "enroll-finger-not-centered" => (Mode::Scan, "Приложите палец к центру сканера"),
            "enroll-duplicate" => (Mode::Error, "Этот отпечаток уже зарегистрирован."),
            "enroll-data-full" => (Mode::Error, "Хранилище отпечатков заполнено."),
            "enroll-disconnected" => (Mode::Error, "Сканер отключился."),
            _ if done => (
                Mode::Error,
                "Регистрация не завершилась. Повторите попытку.",
            ),
            _ => (Mode::Scan, "Приложите палец ещё раз"),
        };
        if done {
            self.end_enrollment(mode, message.into());
        } else if self.enrollment.borrow().is_some() {
            self.show(mode, message);
        }
    }

    fn end_enrollment(self: &Rc<Self>, mode: Mode, message: String) {
        {
            let mut current = self.enrollment.borrow_mut();
            let Some(active) = current.as_mut() else {
                return;
            };
            if !active.started {
                active.pending_result = Some((mode, message));
                return;
            }
        }
        let Some(active) = self.enrollment.borrow_mut().take() else {
            return;
        };
        self.show(mode, &message);
        self.cancel.set_visible(false);
        let view = self.clone();
        glib::spawn_future_local(async move {
            active.reader.unwatch(active.owner_watch);
            drop(active._statuses);
            let _ = active.reader.call("EnrollStop", None).await;
            let _ = active.reader.call("Release", None).await;
            view.complete(mode, message);
        });
    }
}

fn finger_label(id: &str) -> &str {
    FINGERS
        .iter()
        .find(|(finger, _)| *finger == id)
        .map_or(id, |(_, label)| label)
}

fn main() -> glib::ExitCode {
    if gio::Credentials::new()
        .unix_user()
        .is_ok_and(|uid| uid == 0)
    {
        eprintln!("Запускайте приложение из графического сеанса обычного пользователя.");
        return glib::ExitCode::FAILURE;
    }
    glib::set_application_name("HonorKit");
    let app = gtk::Application::builder()
        .application_id("io.github.no1se.HonorKit")
        .build();
    app.connect_startup(|_| {
        let css = gtk::CssProvider::new();
        css.load_from_data(include_str!("style.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &css,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
        } else {
            View::new(app);
        }
    });
    app.run()
}
