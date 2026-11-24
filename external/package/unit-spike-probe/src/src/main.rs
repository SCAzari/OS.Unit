//! Toolkit spike (UNIT-OS-WP-0010, issue #8): reference screen reader and switch scanner over the
//! AT-SPI accessibility bus, the bus the Unit screen reader and switch control will use (Software
//! Architecture §4, accessibility services). It checks a running spike screen against
//! spike/screens.json and prints PROBE lines for the host's results record.
//!
//!   unit-spike-probe dump <screen>          C1: every element exposed with role, name, state or
//!                                           value, in reading order; each is spoken (espeak)
//!   unit-spike-probe focus <seconds>        C1/C4a: announce each focus change (toolkit focus chain)
//!   unit-spike-probe scan <screen>          C4b: compositor-level scanner over AT-SPI
//!   unit-spike-probe text <name>            C2: read back the text of an entry
//!   unit-spike-probe wait <screen> <secs>   wait until the screen's window is on the bus
//!   unit-spike-probe extents <screen> <name>...  element positions (points) for the host's taps
//!
//! Screens: sign-in, settings, display. The spec is read from $UNIT_SPIKE_SPEC or
//! /usr/share/unit-spike/screens.json.

use atspi_common::{CoordType, Interface, RelationType, State};
use atspi_proxies::{
    accessible::AccessibleProxy,
    action::ActionProxy,
    bus::BusProxy,
    component::ComponentProxy,
    registry::RegistryProxy,
    text::TextProxy,
    value::ValueProxy,
};
use futures_lite::StreamExt;
use serde_json::Value as Json;
use std::time::{Duration, Instant};
use zbus::Connection;

type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug)]
struct Node {
    bus: String,
    path: String,
    depth: usize,
    role: String,
    name: String,
    focusable: bool,
    focused: bool,
    showing: bool,
    checked: bool,
    editable: bool,
    value: Option<f64>,
    has_action: bool,
    has_component: bool,
    has_text: bool,
    flows_to: Vec<(String, String)>,
}

fn title(screen: &str) -> &'static str {
    match screen {
        "sign-in" => "Sign in",
        "display" => "Display and appearance",
        _ => "Settings",
    }
}

fn spec() -> Res<Json> {
    let path = std::env::var("UNIT_SPIKE_SPEC").unwrap_or_else(|_| "/usr/share/unit-spike/screens.json".into());
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

async fn a11y_bus() -> Res<Connection> {
    let session = Connection::session().await?;
    let address = BusProxy::new(&session).await?.get_address().await?;
    Ok(zbus::connection::Builder::address(address.as_str())?.build().await?)
}

async fn proxy<'a>(conn: &'a Connection, bus: &str, path: &str) -> Res<AccessibleProxy<'a>> {
    Ok(AccessibleProxy::builder(conn)
        .destination(bus.to_owned())?
        .path(path.to_owned())?
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await?)
}

async fn read_node(conn: &Connection, bus: &str, path: &str, depth: usize) -> Res<(Node, Vec<(String, String)>)> {
    let acc = proxy(conn, bus, path).await?;
    let states = acc.get_state().await?;
    let ifaces = acc.get_interfaces().await?;
    let mut value = None;
    if ifaces.contains(Interface::Value) {
        if let Ok(v) = ValueProxy::builder(conn).destination(bus.to_owned())?.path(path.to_owned())?.build().await {
            value = v.current_value().await.ok();
        }
    }
    let mut flows_to = Vec::new();
    if let Ok(rels) = acc.get_relation_set().await {
        for (kind, targets) in rels {
            if kind == RelationType::FlowsTo {
                for t in targets {
                    flows_to.push((t.name_as_str().unwrap_or_default().to_owned(), t.path_as_str().to_owned()));
                }
            }
        }
    }
    let children = acc
        .get_children()
        .await?
        .into_iter()
        .map(|c| (c.name_as_str().unwrap_or(bus).to_owned(), c.path_as_str().to_owned()))
        .collect();
    let node = Node {
        bus: bus.to_owned(),
        path: path.to_owned(),
        depth,
        // GetRoleName, or the numeric role's standard name where an implementation leaves the
        // name empty (AccessKit, used by Slint, does).
        role: match acc.get_role_name().await.unwrap_or_default() {
            r if !r.is_empty() => r,
            _ => acc.get_role().await.map(|r| r.name().to_owned()).unwrap_or_default(),
        },
        name: acc.name().await.unwrap_or_default().trim().to_owned(),
        focusable: states.contains(State::Focusable),
        focused: states.contains(State::Focused),
        showing: states.contains(State::Showing),
        checked: states.contains(State::Checked) || states.contains(State::Pressed),
        editable: states.contains(State::Editable),
        value,
        has_action: ifaces.contains(Interface::Action),
        has_component: ifaces.contains(Interface::Component),
        has_text: ifaces.contains(Interface::Text),
        flows_to,
    };
    Ok((node, children))
}

/// Depth-first walk (the reading order a screen reader follows).
async fn walk(conn: &Connection, bus: &str, path: &str) -> Res<Vec<Node>> {
    let mut out = Vec::new();
    let mut stack = vec![(bus.to_owned(), path.to_owned(), 0usize)];
    while let Some((b, p, d)) = stack.pop() {
        if out.len() > 2000 {
            break;
        }
        let Ok((node, children)) = read_node(conn, &b, &p, d).await else { continue };
        out.push(node);
        for (cb, cp) in children.into_iter().rev() {
            stack.push((cb, cp, d + 1));
        }
    }
    Ok(out)
}

/// The window (frame) of a spike app showing the given screen.
async fn find_window(conn: &Connection, screen: &str) -> Res<Option<(String, String)>> {
    let want = title(screen);
    let root = proxy(conn, "org.a11y.atspi.Registry", "/org/a11y/atspi/accessible/root").await?;
    for app in root.get_children().await? {
        let bus = app.name_as_str().unwrap_or_default().to_owned();
        let Ok(app_proxy) = proxy(conn, &bus, app.path_as_str()).await else { continue };
        let Ok(windows) = app_proxy.get_children().await else { continue };
        for w in windows {
            let wb = w.name_as_str().unwrap_or(&bus).to_owned();
            if let Ok(wp) = proxy(conn, &wb, w.path_as_str()).await {
                if wp.name().await.map(|n| n.trim() == want).unwrap_or(false) {
                    return Ok(Some((wb, w.path_as_str().to_owned())));
                }
            }
        }
    }
    Ok(None)
}

async fn wait_window(conn: &Connection, screen: &str, secs: u64) -> Res<(String, String)> {
    let end = Instant::now() + Duration::from_secs(secs);
    loop {
        if let Some(w) = find_window(conn, screen).await? {
            return Ok(w);
        }
        if Instant::now() > end {
            return Err(format!("no window '{}' on the accessibility bus", title(screen)).into());
        }
        async_io::Timer::after(Duration::from_millis(500)).await;
    }
}

fn kinds(spec: &Json, kind: &str) -> Vec<String> {
    spec["kinds"][kind]
        .as_array()
        .map(|a| a.iter().filter_map(|r| r.as_str().map(str::to_owned)).collect())
        .unwrap_or_default()
}

fn expected(spec: &Json, screen: &str) -> Vec<Json> {
    let mut v: Vec<Json> = spec["status_bar"].as_array().cloned().unwrap_or_default();
    v.extend(spec["screens"][screen]["elements"].as_array().cloned().unwrap_or_default());
    v
}

fn speak(text: &str, n: usize) {
    let dir = "/tmp/unit-spike-speech";
    let _ = std::fs::create_dir_all(dir);
    let ok = std::process::Command::new("espeak")
        .args(["-w", &format!("{}/{:03}.wav", dir, n), text])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    println!("PROBE SAY {} \"{}\"", if ok { "spoken" } else { "not-spoken" }, text);
}

/// Whether an element is a control a person operates (and a switch scanner steps to).
fn interactive(n: &Node) -> bool {
    const ROLES: &[&str] = &["push button", "button", "toggle button", "radio button", "radio menu item",
        "check box", "slider", "spin button", "entry", "password text", "editbar"];
    ROLES.contains(&n.role.as_str()) || (n.role == "text" && n.has_text && n.editable)
}

fn utterance(n: &Node) -> String {
    let mut s = if n.name.is_empty() { "(no name)".to_owned() } else { n.name.clone() };
    s.push_str(", ");
    s.push_str(&n.role);
    if let Some(v) = n.value {
        s.push_str(&format!(", {}", v.round()));
    }
    if n.checked {
        s.push_str(", checked");
    }
    s
}

/// C1: every expected element is exposed with an allowed role, its name and state or value, in
/// the layout's reading order. Each element is spoken as a screen reader would read it.
async fn dump(screen: &str) -> Res<bool> {
    let spec = spec()?;
    let conn = a11y_bus().await?;
    let (bus, path) = wait_window(&conn, screen, 60).await?;
    let nodes = walk(&conn, &bus, &path).await?;
    for n in &nodes {
        println!(
            "PROBE NODE {} {}|{}|{}{}{}{}",
            n.depth,
            n.role,
            n.name,
            if n.focusable { "focusable " } else { "" },
            if n.showing { "showing " } else { "" },
            if n.checked { "checked " } else { "" },
            n.value.map(|v| format!("value={}", v)).unwrap_or_default()
        );
    }
    let mut last = 0usize;
    let (mut found, mut problems) = (0usize, Vec::new());
    let want = expected(&spec, screen);
    for (i, e) in want.iter().enumerate() {
        let name = e["name"].as_str().unwrap_or_default();
        let roles = kinds(&spec, e["kind"].as_str().unwrap_or_default());
        let Some((pos, n)) = nodes.iter().enumerate().find(|(_, n)| n.name == name) else {
            problems.push(format!("missing \"{}\"", name));
            continue;
        };
        found += 1;
        speak(&utterance(n), i);
        if !roles.contains(&n.role) {
            problems.push(format!("\"{}\" has role \"{}\"", name, n.role));
        }
        if e["checked"].as_bool() == Some(true) && !n.checked {
            problems.push(format!("\"{}\" not checked", name));
        }
        if let Some(v) = e["value"].as_f64() {
            match n.value {
                Some(got) if (got - v).abs() < 0.5 => {}
                Some(got) => problems.push(format!("\"{}\" value {} not {}", name, got, v)),
                None => problems.push(format!("\"{}\" has no value", name)),
            }
        }
        if pos < last {
            problems.push(format!("\"{}\" out of reading order", name));
        }
        last = last.max(pos);
    }
    let pass = problems.is_empty();
    println!(
        "PROBE C1 {} {} found={}/{} problems={}",
        screen,
        if pass { "PASS" } else { "FAIL" },
        found,
        want.len(),
        serde_json::to_string(&problems)?
    );
    Ok(pass)
}

/// Announces each focus change for `secs` seconds (toolkit focus chain, condition C4a).
async fn focus(secs: u64) -> Res<bool> {
    let conn = a11y_bus().await?;
    let registry = RegistryProxy::new(&conn).await?;
    registry.register_event("object:state-changed:focused").await?;
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.a11y.atspi.Event.Object")?
        .member("StateChanged")?
        .build();
    let mut stream = zbus::MessageStream::for_match_rule(rule, &conn, Some(64)).await?;
    println!("PROBE FOCUS listening");
    let end = Instant::now() + Duration::from_secs(secs);
    let mut n = 500;
    loop {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            break;
        }
        let next = futures_lite::future::or(async { stream.next().await.map(Some) }, async {
            async_io::Timer::after(left).await;
            None
        })
        .await;
        let Some(Some(Ok(msg))) = next else { break };
        let header = msg.header();
        let body = msg.body();
        let Ok((kind, detail1, _d2, _any, _props)) =
            body.deserialize::<(String, i32, i32, zbus::zvariant::OwnedValue, std::collections::HashMap<String, zbus::zvariant::OwnedValue>)>()
        else {
            continue;
        };
        if kind != "focused" || detail1 != 1 {
            continue;
        }
        let (Some(sender), Some(path)) = (header.sender(), header.path()) else { continue };
        if let Ok((node, _)) = read_node(&conn, sender.as_str(), path.as_str(), 0).await {
            println!("PROBE FOCUS {}|{}", node.role, node.name);
            n += 1;
            speak(&utterance(&node), n);
        }
    }
    Ok(true)
}

/// C4b: a compositor-level scanner works from what the application exposes on AT-SPI. It steps
/// through the focusable, showing elements in the order the application declares there: by
/// flows-to relations if it gives any, otherwise in tree order. Each element is highlighted with
/// Component.GrabFocus, and Select is Action.DoAction(0).
async fn scan(screen: &str) -> Res<bool> {
    let spec = spec()?;
    let conn = a11y_bus().await?;
    let (bus, path) = wait_window(&conn, screen, 60).await?;
    let nodes = walk(&conn, &bus, &path).await?;
    // A switch scanner steps through the controls a person can operate, not static text (some
    // toolkits mark labels focusable on AT-SPI).
    let targets: Vec<&Node> = nodes
        .iter()
        .filter(|n| n.focusable && n.showing && !n.name.is_empty() && interactive(n))
        .collect();
    let uses_flows = targets.iter().any(|n| !n.flows_to.is_empty());
    let mut order: Vec<&Node> = Vec::new();
    if uses_flows {
        // Start at the element nothing flows to, then follow flows-to.
        let pointed: Vec<(String, String)> = targets.iter().flat_map(|n| n.flows_to.clone()).collect();
        let mut cur = targets.iter().find(|n| !pointed.contains(&(n.bus.clone(), n.path.clone()))).copied();
        while let Some(n) = cur {
            if order.iter().any(|o| o.path == n.path) {
                break;
            }
            order.push(n);
            cur = n.flows_to.first().and_then(|(b, p)| targets.iter().find(|t| &t.bus == b && &t.path == p).copied());
        }
    } else {
        order = targets.clone();
    }
    let names: Vec<String> = order.iter().map(|n| n.name.clone()).collect();
    let declared: Vec<String> = spec["screens"][screen]["scan"]
        .as_array()
        .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect())
        .unwrap_or_default();
    println!("PROBE SCAN {} source={} order={}", screen, if uses_flows { "flows-to" } else { "tree-order" }, serde_json::to_string(&names)?);
    let order_ok = names == declared;

    // Highlight each element in turn.
    let mut highlighted = 0;
    for n in &order {
        if !n.has_component {
            continue;
        }
        let comp = ComponentProxy::builder(&conn).destination(n.bus.clone())?.path(n.path.clone())?.build().await?;
        if comp.grab_focus().await.unwrap_or(false) {
            async_io::Timer::after(Duration::from_millis(150)).await;
            if let Ok((now, _)) = read_node(&conn, &n.bus, &n.path, 0).await {
                if now.focused {
                    highlighted += 1;
                }
            }
        }
    }
    println!("PROBE SCAN {} highlighted={}/{}", screen, highlighted, order.len());

    // Select.
    // The scanner's own Select target where the screen has one (sign-in: the toolkit focus-chain
    // test has already pressed a digit), otherwise the screen's select target.
    let select = if spec["screens"][screen]["scan_select"].is_object() {
        &spec["screens"][screen]["scan_select"]
    } else {
        &spec["screens"][screen]["select"]
    };
    let press = select["press"].as_str().unwrap_or_default();
    let mut select_ok = false;
    if let Some(n) = order.iter().find(|n| n.name == press) {
        if n.has_action {
            let action = ActionProxy::builder(&conn).destination(n.bus.clone())?.path(n.path.clone())?.build().await?;
            let done = action.do_action(0).await.unwrap_or(false);
            async_io::Timer::after(Duration::from_millis(800)).await;
            if let Some(expect) = select["expect"].as_str() {
                let after = walk(&conn, &bus, &path).await?;
                select_ok = done && after.iter().any(|a| a.name == expect);
            } else if let Some(next) = select["expect_screen"].as_str() {
                select_ok = done && find_window(&conn, next).await?.is_some();
            }
        }
    }
    let pass = order_ok && highlighted == order.len() && select_ok;
    println!(
        "PROBE C4B {} {} order={} highlighted={}/{} select={}",
        screen,
        if pass { "PASS" } else { "FAIL" },
        if order_ok { "declared" } else { "not-declared" },
        highlighted,
        order.len(),
        if select_ok { "ok" } else { "failed" }
    );
    Ok(pass)
}

/// C2: the text of the element with this name, read through the Text interface.
async fn text(name: &str) -> Res<bool> {
    let conn = a11y_bus().await?;
    let (bus, path) = wait_window(&conn, "settings", 30).await?;
    for n in walk(&conn, &bus, &path).await? {
        if n.name == name && n.has_text {
            let t = TextProxy::builder(&conn).destination(n.bus.clone())?.path(n.path.clone())?.build().await?;
            let count = t.character_count().await?;
            let s = t.get_text(0, count).await?;
            println!("PROBE TEXT \"{}\" text=\"{}\"", name, s);
            return Ok(true);
        }
    }
    println!("PROBE TEXT \"{}\" not found", name);
    Ok(false)
}

/// Position of named elements in window coordinates (points), for the host's taps. The spike
/// window fills the display at its origin, so these are display positions in points.
async fn extents(screen: &str, names: &[String]) -> Res<bool> {
    let conn = a11y_bus().await?;
    let (bus, path) = wait_window(&conn, screen, 60).await?;
    let nodes = walk(&conn, &bus, &path).await?;
    let mut all = true;
    for want in names {
        match nodes.iter().find(|n| &n.name == want && n.has_component) {
            Some(n) => {
                let comp = ComponentProxy::builder(&conn).destination(n.bus.clone())?.path(n.path.clone())?.build().await?;
                let (x, y, w, h) = comp.get_extents(CoordType::Window).await?;
                println!("PROBE EXTENTS \"{}\" {} {} {} {}", want, x, y, w, h);
            }
            None => {
                println!("PROBE EXTENTS \"{}\" not-found", want);
                all = false;
            }
        }
    }
    Ok(all)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
    let result = zbus::block_on(async {
        match arg(0).as_str() {
            "dump" => dump(&arg(1)).await,
            "focus" => focus(arg(1).parse().unwrap_or(30)).await,
            "scan" => scan(&arg(1)).await,
            "text" => text(&arg(1)).await,
            "extents" => extents(&arg(1), &args[2.min(args.len())..]).await,
            "wait" => {
                let conn = a11y_bus().await?;
                wait_window(&conn, &arg(1), arg(2).parse().unwrap_or(60)).await.map(|_| true)
            }
            _ => Err("usage: unit-spike-probe dump|focus|scan|text|wait ...".into()),
        }
    });
    match result {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(e) => {
            println!("PROBE ERROR {}", e);
            std::process::exit(2);
        }
    }
}
