use crate::art::{Art, Mode};
use gtk::{gio, glib, prelude::*};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

const LOGIN: &str = "org.freedesktop.login1";
const PROPS: &str = "org.freedesktop.DBus.Properties";
const SESSION: &str = "org.freedesktop.login1.Session";

#[derive(Clone, Copy)]
pub struct Preferences {
    pub enabled: bool,
    pub scan: bool,
    pub long: bool,
}

impl Preferences {
    pub fn load() -> Self {
        let file = glib::KeyFile::new();
        let _ = file.load_from_file(path(), glib::KeyFileFlags::NONE);
        Self {
            enabled: file.boolean("Unlock", "Enabled").unwrap_or(true),
            scan: file.boolean("Unlock", "Scan").unwrap_or(false),
            long: file.boolean("Unlock", "Long").unwrap_or(false),
        }
    }

    pub fn save(self) -> Result<(), Box<dyn std::error::Error>> {
        let file = glib::KeyFile::new();
        file.set_boolean("Unlock", "Enabled", self.enabled);
        file.set_boolean("Unlock", "Scan", self.scan);
        file.set_boolean("Unlock", "Long", self.long);
        std::fs::create_dir_all(glib::user_config_dir().join("honorkit"))?;
        file.save_to_file(path())?;
        Ok(())
    }
}

fn path() -> std::path::PathBuf {
    glib::user_config_dir().join("honorkit/unlock.ini")
}

#[derive(Default)]
struct UnlockGate {
    locked: bool,
    last_match: Option<Instant>,
    just_unlocked: Option<Instant>,
}

impl UnlockGate {
    fn lock_changed(&mut self, locked: bool, now: Instant) -> bool {
        if self.locked == locked {
            return false;
        }
        self.locked = locked;
        self.just_unlocked = None;
        let recent = self
            .last_match
            .take()
            .is_some_and(|then| now - then <= Duration::from_secs(3));
        if !locked && !recent {
            self.just_unlocked = Some(now);
        }
        !locked && recent
    }

    fn fingerprint_match(&mut self, now: Instant) -> bool {
        if self.locked {
            self.last_match = Some(now);
            false
        } else {
            self.just_unlocked
                .take()
                .is_some_and(|then| now - then <= Duration::from_millis(300))
        }
    }
}

async fn get_property(
    bus: &gio::DBusConnection,
    object: &str,
    interface: &str,
    name: &str,
) -> Result<glib::Variant, glib::Error> {
    let reply = bus
        .call_future(
            Some(LOGIN),
            object,
            PROPS,
            "Get",
            Some(&(interface, name).to_variant()),
            None,
            gio::DBusCallFlags::NONE,
            4_000,
        )
        .await?;
    reply
        .try_get::<(glib::Variant,)>()
        .map(|(value,)| value)
        .map_err(|err| glib::Error::new(gio::IOErrorEnum::InvalidData, &err.to_string()))
}

pub async fn watch(
    app: &gtk::Application,
) -> Result<(gio::SignalSubscription, gio::SignalSubscription), Box<dyn std::error::Error>> {
    let bus = gio::bus_get_future(gio::BusType::System).await?;
    let display = get_property(
        &bus,
        "/org/freedesktop/login1/user/self",
        "org.freedesktop.login1.User",
        "Display",
    )
    .await?;
    let (id, path) = display.try_get::<(String, glib::variant::ObjectPath)>()?;
    if id.is_empty() || path.as_str() == "/" {
        return Err("no graphical logind session".into());
    }
    let locked = get_property(&bus, path.as_str(), SESSION, "LockedHint")
        .await?
        .try_get::<bool>()?;
    let gate = Rc::new(RefCell::new(UnlockGate {
        locked,
        ..Default::default()
    }));
    let lock_gate = gate.clone();
    let lock_app = app.downgrade();
    let lock = bus.subscribe_to_signal(
        Some(LOGIN),
        Some(PROPS),
        Some("PropertiesChanged"),
        Some(path.as_str()),
        Some(SESSION),
        gio::DBusSignalFlags::NONE,
        move |signal| {
            let changes = signal.parameters.child_value(1);
            let changes = glib::VariantDict::new(Some(&changes));
            if let Ok(Some(locked)) = changes.lookup::<bool>("LockedHint")
                && lock_gate.borrow_mut().lock_changed(locked, Instant::now())
            {
                schedule(&lock_app, &lock_gate);
            }
        },
    );
    let finger_app = app.downgrade();
    let finger = bus.subscribe_to_signal(
        Some("net.reactivated.Fprint"),
        Some("net.reactivated.Fprint.Device"),
        Some("VerifyStatus"),
        None,
        None,
        gio::DBusSignalFlags::NONE,
        move |signal| {
            if signal
                .parameters
                .get::<(String, bool)>()
                .is_some_and(|(status, done)| done && status == "verify-match")
                && gate.borrow_mut().fingerprint_match(Instant::now())
            {
                schedule(&finger_app, &gate);
            }
        },
    );
    Ok((lock, finger))
}

fn schedule(app: &glib::WeakRef<gtk::Application>, gate: &Rc<RefCell<UnlockGate>>) {
    let app = app.clone();
    let gate = gate.clone();
    glib::timeout_add_local_once(Duration::from_millis(180), move || {
        let prefs = Preferences::load();
        if !gate.borrow().locked
            && prefs.enabled
            && let Some(app) = app.upgrade()
        {
            flash(&app, prefs, None);
        }
    });
}

pub fn flash(app: &gtk::Application, prefs: Preferences, parent: Option<&gtk::Window>) {
    let window = gtk::Window::builder()
        .application(app)
        .title("HonorKit")
        .decorated(false)
        .resizable(false)
        .focusable(false)
        .default_width(245)
        .build();
    if let Some(parent) = parent {
        window.set_transient_for(Some(parent));
    }
    let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
    card.add_css_class("unlock-card");
    let art = Art::new();
    art.set(if prefs.scan { Mode::Scan } else { Mode::Passed });
    card.append(&art.widget);
    let label = gtk::Label::new(Some("Разблокировано"));
    label.add_css_class("scanner-status");
    card.append(&label);
    window.set_child(Some(&card));

    let allow_motion = gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations());
    let duration = if prefs.long { 1650.0 } else { 1050.0 };
    if allow_motion {
        let started = Instant::now();
        let window = window.downgrade();
        card.set_opacity(0.08);
        card.add_tick_callback(move |card, _| {
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            let opacity = (ms / 150.0).min(1.0) * ((duration - ms) / 280.0).clamp(0.0, 1.0);
            card.set_opacity(opacity);
            if ms >= duration {
                if let Some(window) = window.upgrade() {
                    window.close();
                }
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
    // Also close if the compositor never starts the frame clock.
    let close_window = window.downgrade();
    glib::timeout_add_local_once(Duration::from_millis(duration as u64 + 120), move || {
        if let Some(window) = close_window.upgrade() {
            window.close();
        }
    });
    window.set_visible(true);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_correlated_lock_cycles_trigger_once() {
        let start = Instant::now();
        let mut gate = UnlockGate::default();
        assert!(!gate.fingerprint_match(start));
        assert!(!gate.lock_changed(true, start));
        assert!(!gate.lock_changed(false, start)); // Password, until a match arrives.
        assert!(!gate.fingerprint_match(start + Duration::from_secs(1)));
        assert!(!gate.lock_changed(true, start));
        assert!(!gate.fingerprint_match(start));
        assert!(gate.lock_changed(false, start + Duration::from_millis(100)));
        assert!(!gate.fingerprint_match(start + Duration::from_millis(101)));

        assert!(!gate.lock_changed(true, start));
        assert!(!gate.lock_changed(false, start));
        assert!(gate.fingerprint_match(start + Duration::from_millis(20)));
        assert!(!gate.fingerprint_match(start + Duration::from_millis(21)));

        assert!(!gate.lock_changed(true, start));
        assert!(!gate.fingerprint_match(start));
        assert!(!gate.lock_changed(false, start + Duration::from_secs(4)));
    }
}
