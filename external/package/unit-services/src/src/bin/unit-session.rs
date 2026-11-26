//! unit-session: the Session service (see the library's session module).

use unit_services::{desktop, session};

fn main() -> zbus::Result<()> {
    unit_services::init_logging();
    let dirs = desktop::search_dirs();
    let registry = session::SharedRegistry::default();
    zbus::block_on(async {
        let _conn = zbus::connection::Builder::session()?
            .serve_at(session::OBJECT_PATH, session::SessionService { dirs: dirs.clone(), registry: registry.clone() })?
            .serve_at(session::LIFECYCLE_PATH, session::Lifecycle { dirs, registry })?
            .name(session::BUS_NAME)?
            .build()
            .await?;
        tracing::info!("Session service ready");
        std::future::pending::<()>().await;
        Ok(())
    })
}
