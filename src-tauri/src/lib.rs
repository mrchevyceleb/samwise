mod state;
mod commands;
mod models;
pub mod process;

use commands::claude_code::ClaudeCodeState;
use commands::supabase::SupabaseState;
use commands::worker::WorkerState;
use state::AppState;
use tauri::Manager;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::menu::{MenuBuilder, MenuItemBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .manage(ClaudeCodeState::default())
        .manage(WorkerState::default())
        .manage(SupabaseState::default())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Build system tray
            let show_item = MenuItemBuilder::with_id("show", "Show SamWise").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit SamWise").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&show_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("SamWise - AI Employee")
                .menu(&tray_menu)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Files
            commands::files::read_file_text,
            commands::files::write_file_text,
            commands::files::create_file,
            commands::files::delete_path,
            commands::files::rename_path,
            commands::files::search_files,
            commands::files::get_file_info,
            commands::files::read_directory_tree,
            commands::files::read_directory_children,
            commands::files::list_all_files,
            commands::files::scan_for_repos,
            // Settings
            commands::settings::save_settings,
            commands::settings::load_settings,
            // Git
            commands::git::git_status,
            commands::git::git_diff,
            commands::git::git_diff_staged,
            commands::git::git_stage_file,
            commands::git::git_unstage_file,
            commands::git::git_stage_all,
            commands::git::git_unstage_all,
            commands::git::git_discard_file,
            commands::git::git_commit,
            commands::git::git_log,
            commands::git::git_branch_list,
            commands::git::git_branch_current,
            commands::git::git_checkout,
            commands::git::git_create_branch,
            commands::git::git_stash,
            commands::git::git_stash_pop,
            commands::git::git_push,
            commands::git::git_pull,
            // Claude Code
            commands::claude_code::spawn_claude_code,
            commands::claude_code::write_claude_code,
            commands::claude_code::close_claude_code,
            commands::claude_code::claude_code_prompt,
            // Supabase - Config
            commands::supabase::supabase_set_config,
            commands::supabase::supabase_get_config,
            commands::supabase::supabase_test_connection,
            commands::supabase::supabase_load_doppler,
            // Supabase - Tasks
            commands::supabase::supabase_fetch_tasks,
            commands::supabase::supabase_create_task,
            commands::supabase::supabase_update_task,
            commands::supabase::supabase_delete_task,
            commands::supabase::supabase_claim_task,
            // Supabase - Comments
            commands::supabase::supabase_fetch_comments,
            commands::supabase::supabase_post_comment,
            commands::supabase::supabase_delete_comment,
            // Supabase - Messages (Chat)
            commands::supabase::supabase_fetch_messages,
            commands::supabase::supabase_send_message,
            // Supabase - Crons
            commands::supabase::supabase_fetch_crons,
            commands::supabase::supabase_create_cron,
            commands::supabase::supabase_update_cron,
            // Supabase - Triggers
            commands::supabase::supabase_fetch_triggers,
            commands::supabase::supabase_create_trigger,
            commands::supabase::supabase_update_trigger,
            // Supabase - Projects
            commands::supabase::supabase_fetch_projects,
            commands::supabase::supabase_create_project,
            commands::supabase::supabase_update_project,
            commands::supabase::supabase_delete_project,
            // Supabase - Artifacts
            commands::supabase::supabase_create_artifact,
            commands::supabase::supabase_fetch_artifacts,
            // Supabase - Workers
            commands::supabase::supabase_worker_heartbeat,
            commands::supabase::supabase_check_active_worker,
            commands::supabase::supabase_worker_offline,
            // Worker
            commands::worker::worker_start,
            commands::worker::worker_stop,
            commands::worker::worker_status,
            // Playwright
            commands::playwright::playwright_screenshot,
            commands::playwright::playwright_screenshot_mobile,
            // Health checks
            commands::health::check_claude_code,
            commands::health::check_gh_auth,
            commands::health::check_doppler,
            // Hello
            commands::hello::hello,
            // Chat (direct - no worker dependency)
            commands::chat::chat_respond,
        ])
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    if window.label() == "main" {
                        // Prevent actual close, hide to system tray instead
                        api.prevent_close();
                        let _ = window.hide();
                        log::info!("[app] Main window hidden to system tray");
                    }
                }
                tauri::WindowEvent::Destroyed => {
                    let label = window.label().to_string();
                    log::info!("[app] Window '{}' destroyed", label);

                    if label == "main" {
                        // Kill all Claude Code processes
                        log::info!("[app] Main window destroyed, cleaning up");
                        let cc_state = window.state::<ClaudeCodeState>();
                        let drained: Vec<_> = {
                            let mut processes = cc_state.processes.lock();
                            processes.drain().collect()
                        };
                        for (_, mut proc) in drained {
                            proc.alive.store(false, std::sync::atomic::Ordering::Relaxed);
                            drop(proc.stdin.take());
                            let _ = proc.child.kill();
                            let _ = proc.child.wait();
                        }

                        // Stop the worker loop
                        let worker_state = window.state::<WorkerState>();
                        worker_state
                            .running
                            .store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running SamWise");
}
