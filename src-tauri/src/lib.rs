pub mod domain;

use std::{sync::Arc, time::Duration};

use domain::{
    ids::StableId,
    patch::{FixtureDefinition, FixtureInstance, Patch},
    persistence::{QuicksaveSummary, ShowSummary},
    service::{BackupSummary, MvrImportResult, ServiceResult, SessionStatus, ShowService, now_ms},
};
use tauri::{Manager, State};

type Service<'a> = State<'a, Arc<ShowService>>;

#[tauri::command]
fn list_shows(service: Service) -> ServiceResult<Vec<ShowSummary>> {
    service.list_shows()
}

#[tauri::command]
fn new_show(service: Service, name: String) -> ServiceResult<SessionStatus> {
    service.new_show(&name, now_ms())
}

#[tauri::command]
fn open_show(service: Service, id: String) -> ServiceResult<SessionStatus> {
    service.open_show(&id, now_ms())
}

#[tauri::command]
fn session_status(service: Service) -> ServiceResult<Option<SessionStatus>> {
    service.status()
}

#[tauri::command]
fn save_show(service: Service) -> ServiceResult<SessionStatus> {
    service.save(now_ms())
}

#[tauri::command]
fn save_show_as(service: Service, name: String) -> ServiceResult<SessionStatus> {
    service.save_as(&name, now_ms())
}

#[tauri::command]
fn rename_show(service: Service, name: String) -> ServiceResult<SessionStatus> {
    service.rename(&name)
}

#[tauri::command]
fn quicksave(service: Service, label: String) -> ServiceResult<Vec<QuicksaveSummary>> {
    service.quicksave(&label, now_ms())
}

#[tauri::command]
fn list_quicksaves(service: Service) -> ServiceResult<Vec<QuicksaveSummary>> {
    service.list_quicksaves()
}

#[tauri::command]
fn restore_quicksave(service: Service, id: String) -> ServiceResult<SessionStatus> {
    service.restore_quicksave(&id)
}

#[tauri::command]
fn delete_quicksave(service: Service, id: String) -> ServiceResult<Vec<QuicksaveSummary>> {
    service.delete_quicksave(&id)
}

#[tauri::command]
fn list_backups(service: Service) -> ServiceResult<Vec<BackupSummary>> {
    service.list_backups()
}

#[tauri::command]
fn restore_backup(service: Service, id: String) -> ServiceResult<SessionStatus> {
    service.restore_backup(&id)
}

#[tauri::command]
fn get_patch(service: Service) -> ServiceResult<Patch> {
    service.get_patch()
}

#[tauri::command]
fn save_fixtures(
    service: Service,
    definitions: Vec<FixtureDefinition>,
    fixtures: Vec<FixtureInstance>,
) -> ServiceResult<Patch> {
    service.save_fixtures(definitions, fixtures)
}

#[tauri::command]
fn remove_fixtures(service: Service, ids: Vec<StableId>) -> ServiceResult<Patch> {
    service.remove_fixtures(ids)
}

#[tauri::command]
fn list_profiles(service: Service) -> ServiceResult<Vec<FixtureDefinition>> {
    service.list_profiles()
}

#[tauri::command]
fn import_gdtf(service: Service, path: String) -> ServiceResult<FixtureDefinition> {
    service.import_gdtf_file(path.as_ref())
}

#[tauri::command]
fn import_mvr(service: Service, path: String) -> ServiceResult<MvrImportResult> {
    service.import_mvr_file(path.as_ref())
}

#[tauri::command]
fn export_mvr(service: Service, path: String) -> ServiceResult<()> {
    service.export_mvr_file(path.as_ref())
}

#[tauri::command]
fn export_patch_pdf(service: Service, path: String) -> ServiceResult<()> {
    service.export_patch_pdf_file(path.as_ref())
}

#[tauri::command]
fn get_workspace(service: Service) -> ServiceResult<Option<serde_json::Value>> {
    service.get_workspace()
}

#[tauri::command]
fn set_workspace(service: Service, workspace: serde_json::Value) -> ServiceResult<SessionStatus> {
    service.set_workspace(workspace)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      let data_dir = app.path().app_data_dir()?;
      let service = Arc::new(
        ShowService::new(data_dir.join("shows"), env!("CARGO_PKG_VERSION"))
          .with_profiles_dir(data_dir.join("profiles")),
      );
      app.manage(Arc::clone(&service));

      std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(15));
        if let Err(error) = service.tick(now_ms()) {
          log::warn!("automatic quicksave failed: {error}");
        }
      });
      app.handle().plugin(tauri_plugin_dialog::init())?;
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      list_shows,
      new_show,
      open_show,
      session_status,
      save_show,
      save_show_as,
      rename_show,
      quicksave,
      list_quicksaves,
      restore_quicksave,
      delete_quicksave,
      list_backups,
      restore_backup,
      get_patch,
      save_fixtures,
      remove_fixtures,
      list_profiles,
      import_gdtf,
      import_mvr,
      export_mvr,
      export_patch_pdf,
      get_workspace,
      set_workspace
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
