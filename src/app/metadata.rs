use crate::app::macros::json_struct;
use crate::app::manifest::Arch;

json_struct! {
    /// Metadata on an installed app.
    pub struct Metadata {
        /// The app's architecture.
        pub architecture: Arch,

        /// The bucket the app originated from.
        pub bucket: String,
    }
}
