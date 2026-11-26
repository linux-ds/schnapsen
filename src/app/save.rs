pub(crate) use schnapsen::save::{SaveError, SaveStore, Snapshot};

pub(crate) fn for_app() -> SaveStore {
    SaveStore::new(
        adw::glib::user_data_dir()
            .join("schnapsen")
            .join("save.json"),
    )
}
