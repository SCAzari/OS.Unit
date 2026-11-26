//! The Notification service (Software Architecture r1.16 §4): the freedesktop notification
//! interface for applications, and for the Shell UI the notification centre's history, banners
//! and Do not disturb (Interface Design r1.20, SC03).
//!
//! An application calls `Notify`; the service keeps the notification in the centre's history and,
//! unless Do not disturb is on, signals the Shell UI to show a banner. Do not disturb holds the
//! banners back; the notifications still reach the centre. Opening or dismissing one in the centre
//! tells the application, as the freedesktop interface defines. Per-application modes and VR
//! banners come in stage 3.

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use zbus::{interface, object_server::SignalEmitter, zvariant::OwnedValue, Connection};

use crate::store;

/// Notifications kept in the centre; the oldest goes when a new one would pass this.
pub const HISTORY_LIMIT: usize = 100;
/// Longest text kept, in characters, so one application cannot fill the service's memory.
const MAX_SUMMARY: usize = 200;
const MAX_BODY: usize = 2000;
const MAX_NAME: usize = 100;
const MAX_ACTIONS: usize = 6;

/// Why a notification closed (freedesktop notification specification 1.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum CloseReason {
    Dismissed = 2,
    ClosedByApp = 3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    /// (key, label) pairs; the key "default" is the action for opening the notification.
    pub actions: Vec<(String, String)>,
    /// Seconds since 1970, when it arrived or was last replaced.
    pub time: i64,
}

/// What `Notify` gives the centre.
#[derive(Debug, Clone, Default)]
pub struct Incoming {
    pub app_name: String,
    pub replaces_id: u32,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub actions: Vec<String>,
    /// The "transient" hint: show a banner but keep nothing in the history.
    pub transient: bool,
    pub time: i64,
}

fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// The notification centre: history and Do not disturb.
#[derive(Debug, Default)]
pub struct Centre {
    next_id: u32,
    history: VecDeque<Notification>,
    do_not_disturb: bool,
}

/// What became of a new notification.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub notification: Notification,
    /// Show it as a banner now (Do not disturb is off).
    pub banner: bool,
    /// Notifications dropped from the history to make room.
    pub dropped: Vec<u32>,
}

impl Centre {
    pub fn new(do_not_disturb: bool) -> Self {
        Centre { next_id: 1, history: VecDeque::new(), do_not_disturb }
    }

    fn take_id(&mut self) -> u32 {
        let id = self.next_id;
        // 0 means "no notification" in the interface, so skip it when the count wraps.
        self.next_id = self.next_id.checked_add(1).unwrap_or(1);
        id
    }

    pub fn notify(&mut self, n: Incoming) -> Outcome {
        let replaced = (n.replaces_id != 0)
            .then(|| self.history.iter().position(|h| h.id == n.replaces_id))
            .flatten();
        let id = match replaced {
            Some(pos) => self.history[pos].id,
            None => self.take_id(),
        };
        let actions = n
            .actions
            .chunks_exact(2)
            .take(MAX_ACTIONS)
            .map(|pair| (clip(&pair[0], MAX_NAME), clip(&pair[1], MAX_NAME)))
            .collect();
        let notification = Notification {
            id,
            app_name: clip(&n.app_name, MAX_NAME),
            app_icon: clip(&n.app_icon, MAX_NAME),
            summary: clip(&n.summary, MAX_SUMMARY),
            body: clip(&n.body, MAX_BODY),
            actions,
            time: n.time,
        };
        let mut dropped = Vec::new();
        match (replaced, n.transient) {
            (Some(pos), false) => self.history[pos] = notification.clone(),
            (Some(pos), true) => {
                self.history.remove(pos);
            }
            (None, false) => {
                self.history.push_back(notification.clone());
                while self.history.len() > HISTORY_LIMIT {
                    if let Some(old) = self.history.pop_front() {
                        dropped.push(old.id);
                    }
                }
            }
            (None, true) => {}
        }
        Outcome { notification, banner: !self.do_not_disturb, dropped }
    }

    /// Removes a notification from the history; returns it if it was there.
    pub fn remove(&mut self, id: u32) -> Option<Notification> {
        let pos = self.history.iter().position(|n| n.id == id)?;
        self.history.remove(pos)
    }

    pub fn clear(&mut self) -> Vec<u32> {
        self.history.drain(..).map(|n| n.id).collect()
    }

    pub fn history(&self) -> impl DoubleEndedIterator<Item = &Notification> {
        self.history.iter()
    }

    pub fn do_not_disturb(&self) -> bool {
        self.do_not_disturb
    }

    /// Returns whether it changed.
    pub fn set_do_not_disturb(&mut self, on: bool) -> bool {
        std::mem::replace(&mut self.do_not_disturb, on) != on
    }
}

/// Do not disturb is kept across restarts in the profile's state folder.
pub struct DndStore(PathBuf);

impl DndStore {
    pub fn new(path: PathBuf) -> Self {
        DndStore(path)
    }

    pub fn load(&self) -> bool {
        store::read_map(&self.0)
            .ok()
            .and_then(|m| m.get("do-not-disturb").map(|v| v == "true"))
            .unwrap_or(false)
    }

    pub fn save(&self, on: bool) -> std::io::Result<()> {
        let map = [("do-not-disturb".to_string(), on.to_string())].into_iter().collect();
        store::write_atomic(&self.0, &store::format("Unit notification state", &map))
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub type Shared = Arc<Mutex<Centre>>;

pub const BUS_NAME: &str = "org.freedesktop.Notifications";
pub const FDO_PATH: &str = "/org/freedesktop/Notifications";
pub const UNIT_BUS_NAME: &str = "os.unit.Notifications1";
pub const UNIT_PATH: &str = "/os/unit/Notifications1";

/// The freedesktop notification interface, for applications (specification 1.2).
pub struct FdoNotifications {
    pub centre: Shared,
}

#[interface(name = "org.freedesktop.Notifications")]
impl FdoNotifications {
    async fn get_capabilities(&self) -> Vec<&'static str> {
        vec!["actions", "body"]
    }

    /// Expiry times are not used: notifications stay in the centre until dismissed, and the
    /// Shell UI times its banners.
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        _expire_timeout: i32,
        #[zbus(connection)] conn: &Connection,
    ) -> zbus::fdo::Result<u32> {
        let transient = hints
            .get("transient")
            .and_then(|v| bool::try_from(v).ok())
            .unwrap_or(false);
        let outcome = self.centre.lock().unwrap().notify(Incoming {
            app_name: app_name.to_string(),
            replaces_id,
            app_icon: app_icon.to_string(),
            summary: summary.to_string(),
            body: body.to_string(),
            actions,
            transient,
            time: now(),
        });
        let fdo = SignalEmitter::new(conn, FDO_PATH)?;
        for id in &outcome.dropped {
            FdoNotifications::notification_closed(&fdo, *id, CloseReason::Dismissed as u32).await?;
        }
        let unit = SignalEmitter::new(conn, UNIT_PATH)?;
        let n = &outcome.notification;
        if outcome.banner {
            UnitNotifications::banner(&unit, n.id, &n.app_name, &n.app_icon, &n.summary, &n.body).await?;
        }
        UnitNotifications::history_changed(&unit).await?;
        Ok(n.id)
    }

    async fn close_notification(&self, id: u32, #[zbus(connection)] conn: &Connection) -> zbus::fdo::Result<()> {
        let removed = self.centre.lock().unwrap().remove(id).is_some();
        if removed {
            let fdo = SignalEmitter::new(conn, FDO_PATH)?;
            FdoNotifications::notification_closed(&fdo, id, CloseReason::ClosedByApp as u32).await?;
            UnitNotifications::history_changed(&SignalEmitter::new(conn, UNIT_PATH)?).await?;
        }
        Ok(())
    }

    async fn get_server_information(&self) -> (&'static str, &'static str, &'static str, &'static str) {
        ("Unit notifications", "Unit", env!("CARGO_PKG_VERSION"), "1.2")
    }

    #[zbus(signal)]
    async fn notification_closed(emitter: &SignalEmitter<'_>, id: u32, reason: u32) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn action_invoked(emitter: &SignalEmitter<'_>, id: u32, action_key: &str) -> zbus::Result<()>;
}

/// The notification centre interface, version 1, for the Shell UI.
pub struct UnitNotifications {
    pub centre: Shared,
    pub dnd: DndStore,
}

impl UnitNotifications {
    /// Removes a notification the user acted on and tells its application.
    async fn finish(&self, conn: &Connection, id: u32, action: Option<&str>) -> zbus::fdo::Result<bool> {
        let Some(n) = self.centre.lock().unwrap().remove(id) else { return Ok(false) };
        let fdo = SignalEmitter::new(conn, FDO_PATH)?;
        if let Some(action) = action {
            if n.actions.iter().any(|(key, _)| key == action) {
                FdoNotifications::action_invoked(&fdo, id, action).await?;
            }
        }
        FdoNotifications::notification_closed(&fdo, id, CloseReason::Dismissed as u32).await?;
        UnitNotifications::history_changed(&SignalEmitter::new(conn, UNIT_PATH)?).await?;
        Ok(true)
    }
}

#[interface(name = "os.unit.Notifications1")]
impl UnitNotifications {
    /// The notification centre, newest first: id, application, icon, summary, body, time
    /// (seconds since 1970) and the action keys and labels.
    #[allow(clippy::type_complexity)]
    async fn history(&self) -> Vec<(u32, String, String, String, String, i64, Vec<(String, String)>)> {
        self.centre
            .lock()
            .unwrap()
            .history()
            .rev()
            .map(|n| {
                (n.id, n.app_name.clone(), n.app_icon.clone(), n.summary.clone(), n.body.clone(), n.time, n.actions.clone())
            })
            .collect()
    }

    /// The user opened a notification: its application gets its "default" action, and it leaves
    /// the centre. Returns false if it was no longer there.
    async fn open(&self, id: u32, #[zbus(connection)] conn: &Connection) -> zbus::fdo::Result<bool> {
        self.finish(conn, id, Some("default")).await
    }

    /// The user chose one of a notification's actions.
    async fn invoke_action(&self, id: u32, action_key: &str, #[zbus(connection)] conn: &Connection) -> zbus::fdo::Result<bool> {
        self.finish(conn, id, Some(action_key)).await
    }

    /// The user dismissed a notification.
    async fn dismiss(&self, id: u32, #[zbus(connection)] conn: &Connection) -> zbus::fdo::Result<bool> {
        self.finish(conn, id, None).await
    }

    /// The user cleared the centre.
    async fn dismiss_all(&self, #[zbus(connection)] conn: &Connection) -> zbus::fdo::Result<()> {
        let ids = self.centre.lock().unwrap().clear();
        let fdo = SignalEmitter::new(conn, FDO_PATH)?;
        for id in &ids {
            FdoNotifications::notification_closed(&fdo, *id, CloseReason::Dismissed as u32).await?;
        }
        if !ids.is_empty() {
            UnitNotifications::history_changed(&SignalEmitter::new(conn, UNIT_PATH)?).await?;
        }
        Ok(())
    }

    /// Do not disturb: when on, new notifications go to the centre without a banner.
    #[zbus(property)]
    async fn do_not_disturb(&self) -> bool {
        self.centre.lock().unwrap().do_not_disturb()
    }

    #[zbus(property)]
    async fn set_do_not_disturb(&mut self, on: bool) -> zbus::fdo::Result<()> {
        let changed = self.centre.lock().unwrap().set_do_not_disturb(on);
        if changed {
            if let Err(e) = self.dnd.save(on) {
                self.centre.lock().unwrap().set_do_not_disturb(!on);
                return Err(zbus::fdo::Error::Failed(format!("could not save Do not disturb: {}", e)));
            }
        }
        Ok(())
    }

    /// The interface version.
    #[zbus(property)]
    fn version(&self) -> u32 {
        1
    }

    /// Show a banner for a new notification.
    #[zbus(signal)]
    async fn banner(
        emitter: &SignalEmitter<'_>,
        id: u32,
        app_name: &str,
        app_icon: &str,
        summary: &str,
        body: &str,
    ) -> zbus::Result<()>;

    /// The history changed: read it again.
    #[zbus(signal)]
    async fn history_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdir::TestDir;

    fn incoming(summary: &str) -> Incoming {
        Incoming { app_name: "Terminal".into(), summary: summary.into(), ..Default::default() }
    }

    #[test]
    fn ids_count_up_from_one() {
        let mut c = Centre::new(false);
        assert_eq!(c.notify(incoming("a")).notification.id, 1);
        assert_eq!(c.notify(incoming("b")).notification.id, 2);
    }

    #[test]
    fn id_zero_is_never_given() {
        let mut c = Centre::new(false);
        c.next_id = u32::MAX;
        assert_eq!(c.notify(incoming("a")).notification.id, u32::MAX);
        assert_eq!(c.notify(incoming("b")).notification.id, 1);
    }

    #[test]
    fn replacing_keeps_the_id_and_the_place() {
        let mut c = Centre::new(false);
        let first = c.notify(incoming("Downloading 10%")).notification.id;
        c.notify(incoming("other"));
        let again = c.notify(Incoming { replaces_id: first, ..incoming("Downloading 50%") });
        assert_eq!(again.notification.id, first);
        let summaries: Vec<_> = c.history().map(|n| n.summary.as_str()).collect();
        assert_eq!(summaries, ["Downloading 50%", "other"]);
        // An unknown id to replace gives a new notification.
        assert_eq!(c.notify(Incoming { replaces_id: 99, ..incoming("x") }).notification.id, 3);
    }

    #[test]
    fn do_not_disturb_holds_banners_back_but_keeps_history() {
        let mut c = Centre::new(false);
        assert!(c.notify(incoming("a")).banner);
        assert!(c.set_do_not_disturb(true));
        assert!(!c.set_do_not_disturb(true));
        let quiet = c.notify(incoming("b"));
        assert!(!quiet.banner);
        assert_eq!(c.history().count(), 2);
    }

    #[test]
    fn transient_notifications_are_not_kept() {
        let mut c = Centre::new(false);
        let out = c.notify(Incoming { transient: true, ..incoming("volume") });
        assert!(out.banner);
        assert_eq!(c.history().count(), 0);
    }

    #[test]
    fn history_is_limited() {
        let mut c = Centre::new(false);
        for i in 0..HISTORY_LIMIT {
            assert!(c.notify(incoming(&i.to_string())).dropped.is_empty());
        }
        let out = c.notify(incoming("one more"));
        assert_eq!(out.dropped, vec![1]);
        assert_eq!(c.history().count(), HISTORY_LIMIT);
    }

    #[test]
    fn long_text_and_odd_actions_are_clipped() {
        let mut c = Centre::new(false);
        let n = c
            .notify(Incoming {
                summary: "s".repeat(1000),
                body: "b".repeat(10_000),
                actions: vec!["default".into(), "Open".into(), "dangling".into()],
                ..incoming("")
            })
            .notification;
        assert_eq!(n.summary.chars().count(), MAX_SUMMARY);
        assert_eq!(n.body.chars().count(), MAX_BODY);
        assert_eq!(n.actions, vec![("default".to_string(), "Open".to_string())]);
    }

    #[test]
    fn remove_and_clear() {
        let mut c = Centre::new(false);
        c.notify(incoming("a"));
        c.notify(incoming("b"));
        assert!(c.remove(1).is_some());
        assert!(c.remove(1).is_none());
        assert_eq!(c.clear(), vec![2]);
        assert_eq!(c.history().count(), 0);
    }

    #[test]
    fn do_not_disturb_kept_across_restart() {
        let dir = TestDir::new();
        let store = DndStore::new(dir.path().join("notifications"));
        assert!(!store.load());
        store.save(true).unwrap();
        assert!(store.load());
    }

    #[test]
    fn matches_its_interface_files() {
        let centre: Shared = Arc::new(Mutex::new(Centre::new(false)));
        crate::check_interface_file(&FdoNotifications { centre: centre.clone() }, "org.freedesktop.Notifications.xml");
        let dir = TestDir::new();
        let unit = UnitNotifications { centre, dnd: DndStore::new(dir.path().join("n")) };
        crate::check_interface_file(&unit, "os.unit.Notifications1.xml");
    }
}
