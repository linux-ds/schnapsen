use adw::{gio, glib, prelude::*};

const FRONT: &str = "card-front";
const BACK: &str = "card-back";
const LEGAL: &str = "highlight-legal-cards";
const OPPONENT_EYES: &str = "show-opponent-eyes";

#[derive(Clone)]
pub(crate) struct Preferences(gio::Settings);

impl Preferences {
    pub fn new() -> Self {
        let installed = gio::SettingsSchemaSource::default();
        let schema = installed
            .as_ref()
            .and_then(|source| source.lookup(super::APP_ID, true))
            .or_else(|| {
                // Run directly from Cargo's build tree before Meson installation.
                gio::SettingsSchemaSource::from_directory(env!("OUT_DIR"), None, false)
                    .ok()
                    .and_then(|source| source.lookup(super::APP_ID, false))
            })
            .expect("Schnapsen's GSettings schema is missing");
        Self(gio::Settings::new_full(
            &schema,
            gio::SettingsBackend::NONE,
            None,
        ))
    }

    pub fn front_id(&self) -> glib::GString {
        self.0.string(FRONT)
    }

    pub fn back_id(&self) -> glib::GString {
        self.0.string(BACK)
    }

    pub fn show_legal(&self) -> bool {
        self.0.boolean(LEGAL)
    }

    pub fn show_opponent_eyes(&self) -> bool {
        self.0.boolean(OPPONENT_EYES)
    }

    pub fn set_front_id(&self, id: &str) {
        write(FRONT, self.0.set_string(FRONT, id));
    }

    pub fn set_back_id(&self, id: &str) {
        write(BACK, self.0.set_string(BACK, id));
    }

    pub fn set_show_legal(&self, value: bool) {
        write(LEGAL, self.0.set_boolean(LEGAL, value));
    }

    pub fn set_show_opponent_eyes(&self, value: bool) {
        write(OPPONENT_EYES, self.0.set_boolean(OPPONENT_EYES, value));
    }

    pub fn connect_changed(&self, changed: impl Fn(&Self) + 'static) -> glib::SignalHandlerId {
        self.0.connect_changed(None, move |settings, _| {
            changed(&Self(settings.clone()));
        })
    }
}

fn write(key: &str, result: Result<(), glib::BoolError>) {
    if let Err(error) = result {
        eprintln!("Could not save the preference {key}: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, process::Command, time::SystemTime};

    #[test]
    fn restores_personal_preferences_across_processes() {
        const MODE: &str = "SCHNAPSEN_SETTINGS_TEST_MODE";
        match env::var(MODE).as_deref() {
            Ok("write") => {
                let settings = Preferences::new();
                assert_eq!(settings.front_id(), "double-german");
                assert_eq!(settings.back_id(), "legacy");
                assert!(!settings.show_legal());
                assert!(!settings.show_opponent_eyes());
                assert!(settings.0.boolean("audio-enabled"));
                settings.set_front_id("french");
                settings.set_back_id("alpine");
                settings.set_show_legal(true);
                settings.set_show_opponent_eyes(true);
                gio::Settings::sync();
            }
            Ok("read") => {
                let settings = Preferences::new();
                assert_eq!(settings.front_id(), "french");
                assert_eq!(settings.back_id(), "alpine");
                assert!(settings.show_legal());
                assert!(settings.show_opponent_eyes());
            }
            _ => {
                let nonce = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                let directory = env::temp_dir()
                    .join(format!("schnapsen-settings-{}-{nonce}", std::process::id()));
                fs::create_dir(&directory).unwrap();
                for mode in ["write", "read"] {
                    let output = Command::new(env::current_exe().unwrap())
                        .args([
                            "--exact",
                            "app::settings::tests::restores_personal_preferences_across_processes",
                        ])
                        .env(MODE, mode)
                        .env("GSETTINGS_BACKEND", "keyfile")
                        .env("XDG_CONFIG_HOME", &directory)
                        .output()
                        .unwrap();
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    assert!(
                        output.status.success() && stdout.contains("1 passed"),
                        "{}\n{}",
                        stdout,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
                fs::remove_dir_all(directory).unwrap();
            }
        }
    }
}
