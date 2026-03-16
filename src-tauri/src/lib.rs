mod state;
mod commands;
mod preview;
mod models;

use commands::claude_code::ClaudeCodeState;
use state::{AppState, StdioMcpState, TerminalState};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .manage(StdioMcpState::default())
        .manage(TerminalState::default())
        .manage(ClaudeCodeState::default())
        .manage(parking_lot::Mutex::new(preview::orchestrator::PreviewOrchestrator::new()))
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
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
            commands::files::read_directory_tree,
            commands::files::read_directory_children,
            commands::files::read_file_text,
            commands::files::write_file_text,
            commands::files::create_file,
            commands::files::delete_path,
            commands::files::rename_path,
            commands::files::import_paths,
            commands::files::search_files,
            commands::files::get_file_info,
            commands::files::list_all_files,
            // Preview
            commands::preview::create_preview_webview,
            commands::preview::set_preview_bounds,
            commands::preview::hide_preview_webview,
            commands::preview::show_preview_webview,
            commands::preview::navigate_preview_webview,
            commands::preview::reload_preview_webview,
            commands::preview::open_preview_devtools,
            commands::preview::close_preview_devtools,
            commands::preview::close_preview_webview,
            commands::preview::preview_check_http,
            // Settings
            commands::settings::save_settings,
            commands::settings::load_settings,
            // AI Streaming
            commands::ai::ai_chat_stream,
            commands::ai::ai_chat_stream_anthropic,
            commands::ai::ai_chat_stream_openai_codex,
            commands::ai::ai_chat_complete,
            commands::ai::ai_fetch_models,
            // AI OAuth
            commands::ai::ai_exchange_openrouter_oauth_code,
            commands::ai::ai_openai_device_start,
            commands::ai::ai_openai_device_poll,
            commands::ai::ai_openai_exchange_authorization_code,
            commands::ai::ai_openai_refresh_oauth_token,
            // Command Execution
            commands::ai::run_command_sync,
            // Chat Session Persistence
            commands::ai::save_chat_session,
            commands::ai::load_chat_session,
            commands::ai::list_chat_sessions,
            commands::ai::delete_chat_session,
            // MCP (HTTP)
            commands::mcp::mcp_list_tools,
            commands::mcp::mcp_call_tool,
            // MCP (Stdio)
            commands::mcp::stdio_mcp_spawn,
            commands::mcp::stdio_mcp_stop,
            commands::mcp::stdio_mcp_list_tools,
            commands::mcp::stdio_mcp_call_tool,
            commands::mcp::stdio_mcp_status,
            // Terminal (PTY)
            commands::terminal::spawn_terminal,
            commands::terminal::write_terminal,
            commands::terminal::resize_terminal,
            commands::terminal::kill_terminal,
            commands::terminal::list_terminals,
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
            // Preview Orchestrator
            commands::orchestrator::preview_open_project,
            commands::orchestrator::preview_stop,
            commands::orchestrator::preview_get_url,
            commands::orchestrator::preview_get_tier,
            commands::orchestrator::preview_rebuild,
            commands::orchestrator::preview_detect_tier,
            commands::orchestrator::preview_scan_env_keys,
            commands::orchestrator::preview_save_env_file,
            commands::orchestrator::preview_load_env_file,
            // Doppler
            commands::doppler::doppler_fetch_workplaces,
            commands::doppler::doppler_fetch_projects,
            commands::doppler::doppler_fetch_configs,
            commands::doppler::doppler_fetch_secrets,
            // Window management
            commands::window::open_folder_in_new_window,
            commands::window::open_path_in_new_window,
            commands::window::git_clone_repo,
        ])
        .on_window_event(|window, event| {
            // When the main window is destroyed, clean up all managed processes
            if let tauri::WindowEvent::Destroyed = event {
                let label = window.label().to_string();
                log::info!("[app] Window '{}' destroyed", label);

                if label == "main" {
                    log::info!("[app] Main window destroyed, cleaning up preview");
                    let state = window.state::<parking_lot::Mutex<preview::orchestrator::PreviewOrchestrator>>();
                    let mut orchestrator = state.lock();
                    *orchestrator = preview::orchestrator::PreviewOrchestrator::new();

                    // Kill all Claude Code processes
                    log::info!("[app] Main window destroyed, cleaning up Claude Code processes");
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
                } else {
                    log::info!("[app] Secondary window '{}' closed", label);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Banana Code");
}
