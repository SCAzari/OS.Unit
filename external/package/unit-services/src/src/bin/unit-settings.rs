//! unit-settings: the Settings service (see the library's settings module).

use unit_services::{settings, store};

fn main() -> zbus::Result<()> {
    unit_services::init_logging();
    let path = store::config_dir().join("settings");
    let service = settings::SettingsService::new(settings::Settings::load(path));
    zbus::block_on(async {
        let _conn = zbus::connection::Builder::session()?
            .serve_at(settings::OBJECT_PATH, service)?
            .name(settings::BUS_NAME)?
            .build()
            .await?;
        tracing::info!("Settings service ready");
        std::future::pending::<()>().await;
        Ok(())
    })
}
