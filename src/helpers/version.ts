// the version shown in the ui, tauri and cargo only accept three part versions so the fourth part lives here
// a.b.c.d maps to semver a.b.(c * 100 + d), so 8.1.1.0 is 8.1.100 in tauri.conf.json, package.json and Cargo.toml
export const APP_VERSION = '8.1.1.0'
