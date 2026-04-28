fn main() {
  for key in [
    "SB_URL",
    "SB_ANON_KEY",
    "SB_SERVICE_ROLE_KEY",
    "TELEGRAM_BOT_TOKEN",
    "TELEGRAM_CHAT_ID",
    "SAM_CALLBACK_SECRET",
  ] {
    println!("cargo:rerun-if-env-changed={}", key);
  }
  tauri_build::build()
}
