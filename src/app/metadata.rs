use crate::app::manifest::Arch;
use crate::json::json_struct;

json_struct! {
    /// Metadata on an installed app.
    pub struct Metadata {
        /// The app's architecture.
        pub architecture: Arch,

        /// The bucket the app originated from.
        pub bucket: String,
    }
}
