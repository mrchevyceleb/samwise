mod state;
mod commands;
mod models;

use commands::claude_code::ClaudeCodeState;
use commands::supabase::SupabaseState;
use commands::worker::WorkerState;
use state::AppState;
use tauri::Manager;

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
            // Supabase - Workers
            commands::supabase::supabase_worker_heartbeat,
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
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
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
        })
        .run(tauri::generate_context!())
        .expect("error while running Agent One");
}
