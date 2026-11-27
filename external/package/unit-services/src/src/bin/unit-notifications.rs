//! unit-notifications: the Notification service (see the library's notifications module).

use std::sync::{Arc, Mutex};

use unit_services::{notifications as n, store};

fn main() -> zbus::Result<()> {
    unit_services::init_logging();
    let dnd = n::DndStore::new(store::state_dir().join("notifications"));
    let centre: n::Shared = Arc::new(Mutex::new(n::Centre::new(dnd.load())));
    zbus::block_on(async {
        let _conn = zbus::connection::Builder::session()?
            .serve_at(n::FDO_PATH, n::FdoNotifications { centre: centre.clone() })?
            .serve_at(n::UNIT_PATH, n::UnitNotifications { centre, dnd })?
            .name(n::BUS_NAME)?
            .name(n::UNIT_BUS_NAME)?
            .build()
            .await?;
        tracing::info!("Notification service ready");
        std::future::pending::<()>().await;
        Ok(())
    })
}
