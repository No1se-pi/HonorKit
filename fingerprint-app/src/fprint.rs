use gtk::{gio, glib, prelude::*};

const SERVICE: &str = "net.reactivated.Fprint";
const MANAGER: &str = "/net/reactivated/Fprint/Manager";
const DEVICE_IFACE: &str = "net.reactivated.Fprint.Device";

#[derive(Clone)]
pub struct Reader(gio::DBusProxy);

impl Reader {
    pub async fn connect() -> Result<Self, glib::Error> {
        let bus = gio::bus_get_future(gio::BusType::System).await?;
        let reply = bus
            .call_future(
                Some(SERVICE),
                MANAGER,
                "net.reactivated.Fprint.Manager",
                "GetDefaultDevice",
                None,
                None,
                gio::DBusCallFlags::NONE,
                15_000,
            )
            .await?;
        let (path,) = reply
            .try_get::<(glib::variant::ObjectPath,)>()
            .map_err(invalid_reply)?;
        gio::DBusProxy::new_future(
            &bus,
            gio::DBusProxyFlags::NONE,
            None,
            Some(SERVICE),
            path.as_str(),
            DEVICE_IFACE,
        )
        .await
        .map(Self)
    }

    pub fn name(&self) -> String {
        self.0
            .cached_property("name")
            .and_then(|value| value.get())
            .unwrap_or_else(|| "Сканер отпечатков".into())
    }

    pub fn stages(&self) -> u32 {
        self.0
            .cached_property("num-enroll-stages")
            .and_then(|value| value.get::<i32>())
            .unwrap_or_default()
            .max(0) as u32
    }

    pub async fn list(&self) -> Result<Vec<String>, glib::Error> {
        match self.call("ListEnrolledFingers", Some("")).await {
            Ok(reply) => reply
                .try_get::<(Vec<String>,)>()
                .map(|(fingers,)| fingers)
                .map_err(invalid_reply),
            Err(error) if has_name(&error, "NoEnrolledPrints") => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    pub async fn call(
        &self,
        method: &str,
        argument: Option<&str>,
    ) -> Result<glib::Variant, glib::Error> {
        let args = argument.map(|value| (value,).to_variant());
        let interactive = matches!(method, "Claim" | "EnrollStart" | "DeleteEnrolledFinger");
        let flags = if interactive {
            gio::DBusCallFlags::ALLOW_INTERACTIVE_AUTHORIZATION
        } else {
            gio::DBusCallFlags::NONE
        };
        self.0
            .call_future(
                method,
                args.as_ref(),
                flags,
                if interactive { 120_000 } else { 20_000 },
            )
            .await
    }

    pub fn watch_enrollment(
        &self,
        status: impl Fn(String, bool) + 'static,
    ) -> gio::SignalSubscription {
        self.0.connection().subscribe_to_signal(
            self.0.name_owner().as_deref(),
            Some(DEVICE_IFACE),
            Some("EnrollStatus"),
            Some(self.0.object_path().as_str()),
            None,
            gio::DBusSignalFlags::NONE,
            move |signal| {
                if let Some((message, done)) = signal.parameters.get::<(String, bool)>() {
                    status(message, done);
                }
            },
        )
    }

    pub fn watch_owner(&self, gone: impl Fn() + 'static) -> glib::SignalHandlerId {
        self.0
            .connect_notify_local(Some("g-name-owner"), move |proxy, _| {
                if proxy.name_owner().is_none() {
                    gone();
                }
            })
    }

    pub fn unwatch(&self, id: glib::SignalHandlerId) {
        self.0.disconnect(id);
    }
}

fn invalid_reply(error: impl std::fmt::Display) -> glib::Error {
    glib::Error::new(gio::IOErrorEnum::InvalidData, &error.to_string())
}

fn has_name(error: &glib::Error, suffix: &str) -> bool {
    gio::DBusError::remote_error(error)
        .is_some_and(|name| name.strip_prefix("net.reactivated.Fprint.Error.") == Some(suffix))
}

pub fn error_text(error: &glib::Error) -> String {
    for (code, text) in [
        ("NoSuchDevice", "Сканер не найден."),
        ("PermissionDenied", "GNOME не разрешил это действие."),
        (
            "AlreadyInUse",
            "Сканер занят. Завершите другую проверку и повторите.",
        ),
        ("NoEnrolledPrints", "Этот отпечаток уже удалён."),
        ("PrintsNotDeleted", "Службе не удалось удалить отпечаток."),
    ] {
        if has_name(error, code) {
            return text.into();
        }
    }
    if error.matches(gio::DBusError::ServiceUnknown)
        || error.matches(gio::DBusError::NameHasNoOwner)
    {
        return "Служба fprintd недоступна.".into();
    }
    eprintln!("fprintd: {error}");
    "Не удалось выполнить действие. Подробности доступны в терминале.".into()
}
