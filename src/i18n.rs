use std::{
    ffi::{c_char, c_int, c_ulong, CStr, CString},
    path::Path,
};

const DOMAIN: &CStr = c"schnapsen";
// LC_MESSAGES on the supported GNU/Linux platform.
const LC_MESSAGES: c_int = 5;

extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn bindtextdomain(domain: *const c_char, directory: *const c_char) -> *mut c_char;
    fn bind_textdomain_codeset(domain: *const c_char, codeset: *const c_char) -> *mut c_char;
    fn dgettext(domain: *const c_char, message: *const c_char) -> *const c_char;
    fn dngettext(
        domain: *const c_char,
        singular: *const c_char,
        plural: *const c_char,
        count: c_ulong,
    ) -> *const c_char;
}

pub fn init() {
    let installed = option_env!("SCHNAPSEN_LOCALEDIR").map(Path::new);
    let locale_dir = installed
        .filter(|path| path.join("de/LC_MESSAGES/schnapsen.mo").is_file())
        .unwrap_or_else(|| Path::new(concat!(env!("OUT_DIR"), "/locale")));
    let directory = CString::new(locale_dir.as_os_str().as_encoded_bytes())
        .expect("the build locale path contains no NUL");
    // Initialize once at startup, before any application threads or widgets.
    // gettext copies the directory and codeset strings.
    unsafe {
        setlocale(LC_MESSAGES, c"".as_ptr());
        bindtextdomain(DOMAIN.as_ptr(), directory.as_ptr());
        bind_textdomain_codeset(DOMAIN.as_ptr(), c"UTF-8".as_ptr());
    }
}

pub fn gettext(message: &str) -> String {
    let message = CString::new(message).expect("message IDs contain no NUL");
    // gettext returns a catalog string or the input message. Copy it before
    // the input CString is dropped.
    unsafe { CStr::from_ptr(dgettext(DOMAIN.as_ptr(), message.as_ptr())) }
        .to_string_lossy()
        .into_owned()
}

pub fn ngettext(singular: &str, plural: &str, count: u32) -> String {
    let singular = CString::new(singular).expect("message IDs contain no NUL");
    let plural = CString::new(plural).expect("message IDs contain no NUL");
    let count = c_ulong::from(count);
    // As with dgettext, the result may refer to one of the input strings.
    unsafe {
        CStr::from_ptr(dngettext(
            DOMAIN.as_ptr(),
            singular.as_ptr(),
            plural.as_ptr(),
            count,
        ))
    }
    .to_string_lossy()
    .into_owned()
}

pub fn format_message(message: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut result = String::with_capacity(message.len());
    let mut remaining = message;
    while let Some((prefix, tail)) = remaining.split_once('{') {
        result.push_str(prefix);
        if let Some((name, rest)) = tail.split_once('}') {
            if let Some((_, value)) = args.iter().find(|(key, _)| *key == name) {
                use std::fmt::Write;
                write!(result, "{value}").expect("writing to String cannot fail");
            } else {
                result.push('{');
                result.push_str(name);
                result.push('}');
            }
            remaining = rest;
        } else {
            result.push('{');
            remaining = tail;
            break;
        }
    }
    result.push_str(remaining);
    result
}

macro_rules! msgid {
    ($message:literal) => {
        $message
    };
}

macro_rules! tr {
    ($message:literal $(,)?) => {
        $crate::i18n::gettext($message)
    };
    ($message:literal, $($key:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::format_message(&$crate::i18n::gettext($message), &[$((stringify!($key), &$value)),+])
    };
}

macro_rules! trn {
    ($singular:literal, $plural:literal, $count:expr $(, $key:ident = $value:expr)* $(,)?) => {{
        let count = $count;
        $crate::i18n::format_message(
            &$crate::i18n::ngettext($singular, $plural, u32::from(count)),
            &[("count", &count) $(, (stringify!($key), &$value))*],
        )
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, process::Command};

    #[test]
    fn locale_selects_messages_and_plural_forms() {
        const MODE: &str = "SCHNAPSEN_LOCALE_TEST";
        match env::var(MODE).as_deref() {
            Ok(language) => {
                init();
                let (preferences, feedback, numbers) = if language == "de" {
                    (
                        "Einstellungen",
                        "Du musst Karo bedienen.",
                        ["0 Spielpunkte", "1 Spielpunkt", "2 Spielpunkte"],
                    )
                } else {
                    (
                        "Preferences",
                        "You must follow suit: Diamonds.",
                        ["0 game points", "1 game point", "2 game points"],
                    )
                };
                assert_eq!(tr!("Preferences"), preferences);
                assert_eq!(
                    tr!("You must follow suit: {suit}.", suit = tr!("Diamonds")),
                    feedback,
                );
                assert_eq!(
                    [0_u32, 1, 2].map(|n| trn!("{count} game point", "{count} game points", n)),
                    numbers,
                );
            }
            Err(_) => {
                for (locale, priorities, expected) in [
                    ("de_AT.UTF-8", "", "de"),
                    ("en_GB.UTF-8", "", "en"),
                    ("en_GB.UTF-8", "fr", "en"),
                    ("de_AT.UTF-8", "en:de", "en"),
                    ("C", "", "en"),
                ] {
                    let output = Command::new(env::current_exe().unwrap())
                        .args([
                            "--exact",
                            "i18n::tests::locale_selects_messages_and_plural_forms",
                        ])
                        .env("LC_ALL", locale)
                        .env("LANGUAGE", priorities)
                        .env(MODE, expected)
                        .output()
                        .unwrap();
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    assert!(
                        output.status.success() && stdout.contains("1 passed"),
                        "{locale} / {priorities}: {stdout}\n{}",
                        String::from_utf8_lossy(&output.stderr),
                    );
                }
            }
        }
    }

    #[test]
    fn translated_templates_reorder_arguments_without_interpreting_values() {
        assert_eq!(
            format_message(
                "{second}, {first}",
                &[("first", &"{second}"), ("second", &2)]
            ),
            "2, {second}",
        );
    }
}
