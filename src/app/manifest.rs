use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, skip_serializing_none, OneOrMany};

/// Macro for generating a JSON enum.
macro_rules! json_enum {
    ($item:item) => {
        #[serde_as]
        #[skip_serializing_none]
        #[derive(Serialize, Deserialize, Debug)]
        #[serde(untagged)]
        $item
    };
}

/// Macro for generating a JSON enum as a map key.
macro_rules! json_enum_key {
    ($item:item) => {
        #[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
        $item
    };
}

/// Macro for generating a JSON struct.
macro_rules! json_struct {
    ($item:item) => {
        #[serde_as]
        #[skip_serializing_none]
        #[derive(Serialize, Deserialize, Debug, Default)]
        $item
    };
}

json_struct! {
    /// A list represented as a single element by itself or multiple elements.
    #[serde(transparent)]
    pub struct List<T>
    where T: DeserializeOwned {
        #[serde_as(deserialize_as = "OneOrMany<_>")]
        pub items: Vec<T>,
    }
}

json_enum! {
    /// A software license for an app.
    pub enum License {
        /// A simple identifier.
        ID(String),

        /// An extended identifier with a URL.
        Extended {
            identifier: Option<String>,
            url: Option<String>,
        },
    }
}

impl Default for License {
    fn default() -> Self {
        Self::ID("Unknown".to_owned())
    }
}

json_enum! {
    /// A pattern for checking versions against the homepage.
    pub enum Checkver {
        /// A regular expression pattern.
        Regex(String),

        /// An extended pattern.
        Extended {
            /// The Github URL to check instead of the homepage.
            github: Option<String>,

            /// The URL to check instead of the homepage.
            url: Option<String>,

            /// The regular expression to check with.
            /// If `jsonpath`, `xpath`, or `script` are not None, the regex is used on their output.
            #[serde(alias = "re")]
            regex: Option<String>,

            /// The JSONPath expression to check with.
            #[serde(alias = "jp")]
            jsonpath: Option<String>,

            /// The XPath expression to check with.
            xpath: Option<String>,

            /// If true, find the last occurance using `regex`.
            /// The regular expression must not be None.
            reverse: Option<bool>,

            /// The user agent to use when fetching the URL.
            useragent: Option<String>,

            /// A PowerShell script to execute to obtain the version.
            script: Option<List<String>>,
        },
    }
}

json_struct! {
    /// A set of instructions for installing an app.
    /// Either file or script must not be None.
    pub struct Installer {
        /// The executable to run.
        pub file: Option<String>,

        /// The PowerShell script to run instead of a file.
        pub script: Option<List<String>>,

        /// The arguments to pass to the executable.
        pub args: Option<List<String>>,

        /// Whether or not to keep the executable.
        pub keep: Option<bool>,
    }
}

json_struct! {
    /// A persistence entry with renaming.
    pub struct PersistEntryRename(
        /// The original path.
        pub String,
        /// The renamed path.
        pub String,
    );
}

json_enum! {
    /// A persistence entry.
    pub enum PersistEntry {
        /// A path to persist.
        Path(String),

        /// A path to rename and persist.
        Extended(PersistEntryRename),
    }
}

/// Files or directories to persist across updates.
/// They are copied from the install directory to the data directory and are symlinked back.
pub type Persist = List<PersistEntry>;

json_struct! {
    /// A PowerShell module to install an app as.
    pub struct PSModule {
        /// The name of the module. This must match the name of a file in the install directory.
        pub name: String,
    }
}

json_struct! {
    /// A desktop shortcut in the Start Menu.
    ///
    /// This is represented as a JSON array of `[executable, name, (parameters), (icon)]`.
    #[derive(Clone)]
    #[serde(try_from = "Vec<String>")]
    #[serde(into = "Vec<String>")]
    pub struct Shortcut {
        /// The path to the executable, relative to the install directory.
        pub executable: String,
        /// The name of the shortcut.
        pub name: String,
        /// The arguments to pass to the executable.
        pub arguments: Option<String>,
        /// The path to the shortcut icon.
        pub icon: Option<String>,
    }
}

impl TryFrom<Vec<String>> for Shortcut {
    type Error = &'static str;

    fn try_from(vec: Vec<String>) -> Result<Self, Self::Error> {
        match vec.len() {
            2..=4 => Ok(Self {
                executable: vec[0].clone(),
                name: vec[1].clone(),
                arguments: vec.get(2).cloned(),
                icon: vec.get(3).cloned(),
            }),
            _ => Err("Shortcut must consist of [executable, name, (parameters), (icon)]"),
        }
    }
}

impl From<Shortcut> for Vec<String> {
    fn from(shortcut: Shortcut) -> Self {
        let mut vec = vec![shortcut.executable, shortcut.name];

        if let Some(parameters) = shortcut.arguments {
            vec.push(parameters);
        }

        if let Some(icon) = shortcut.icon {
            // Ensure parameters are specified as empty.
            vec.resize(3, "".to_owned());
            vec.push(icon)
        }

        vec
    }
}

json_struct! {
    /// An aliased shim for an executable.
    ///
    /// The vec must have 2 or more elements:
    #[derive(Clone)]
    #[serde(try_from = "Vec<String>")]
    #[serde(into = "Vec<String>")]
    pub struct Shim {
        /// The path to the executable, relative to the install directory.
        pub executable: String,
        /// The name of the shim.
        pub name: String,
        /// Arguments to pass to the executable. This may be empty.
        pub arguments: Vec<String>,
    }
}

impl TryFrom<Vec<String>> for Shim {
    type Error = &'static str;

    fn try_from(vec: Vec<String>) -> Result<Self, Self::Error> {
        if vec.len() >= 2 {
            Ok(Self {
                executable: vec[0].clone(),
                name: vec[1].clone(),
                arguments: vec[2..].to_vec(),
            })
        } else {
            Err("Shim must consist of [executable, alias, (args...)]")
        }
    }
}

impl From<Shim> for Vec<String> {
    fn from(shim: Shim) -> Self {
        [vec![shim.executable, shim.name], shim.arguments].concat()
    }
}

json_enum! {
    /// An executable to add to the user's path.
    pub enum Bin {
        /// A path to an executable.
        Path(String),

        /// A shim consisting of a path to an executable, and arguments to pass to the executable.
        Shim(Shim),
    }
}

json_enum_key! {
    /// The mode to use when extracting hashes.
    pub enum HashExtractionMode {
        /// Download the app and hash it locally.
        #[serde(rename = "download")]
        Download,

        /// Extract the hash from the URL directly.
        #[serde(rename = "extract")]
        Extract,

        /// Parse the URL's content as JSON and use the JSONPath expression to obtain the hash.
        #[serde(rename = "json")]
        Json,

        /// Parse the URL's content as XML and use the XPath expression to obtain the hash.
        #[serde(rename = "xpath")]
        Xpath,

        /// Parse the URL's content according to the Resource Description Framework to obtain the hash.
        #[serde(rename = "rdf")]
        Rdf,

        /// Parse the URL's content according to the Metalink file metadata format to obtain the hash.
        #[serde(rename = "metalink")]
        Metalink,

        /// Parse a FossHub link to obtain the hash.
        /// This is implied for FossHub URLs.
        #[serde(rename = "fosshub")]
        Fosshub,

        /// Parse a SourceForge link to obtain the hash.
        /// This is implied for SourceForge URLs.
        #[serde(rename = "sourceforge")]
        Sourceforge,
    }
}

impl Default for HashExtractionMode {
    fn default() -> Self {
        Self::Extract
    }
}

json_struct! {
    /// A set of instructions for extracting an app's hash.
    pub struct HashExtraction {
        /// The regular expression to extract with.
        /// `mode` must be `HashExtraction Mode::Extract`.
        #[serde(alias = "find")]
        pub regex: Option<String>,

        /// The JSONPath expression to extract with.
        /// `mode` must be `HashExtraction Mode::Json`.
        #[serde(alias = "jp")]
        pub jsonpath: Option<String>,

        /// The XPath expression to extract with.
        /// `mode` must be `HashExtraction Mode::Xpath`.
        pub xpath: Option<String>,

        /// The extraction mode.
        pub mode: Option<HashExtractionMode>,

        /// The template URL to extract from.
        pub url: Option<String>,
    }
}

json_enum_key! {
    /// An enum of supported architectures.
    pub enum Arch {
        // 32-bit architecture on x86.
        #[serde(rename = "32bit")]
        X86,

        // 64-bit architecture on x86-64.
        #[serde(rename = "64bit")]
        X86_64,

        // 64-bit architecture on ARM.
        #[serde(rename = "arm64")]
        Arm64,
    }
}

json_struct! {
    /// A subset of an app's manifest that can be customized per architecture in an autoupdate template.
    pub struct AutoupdateArch {
        /// A list of executables to add to the user's path.
        pub bin: Option<List<Bin>>,

        /// A list of directories to add to the user's path, relative to the install directory.
        pub env_add_path: Option<List<String>>,

        /// Set environment variables for the user.
        pub env_set: Option<HashMap<String, String>>,

        /// If an archive is downloaded from the URL, extract a specific directory.
        pub extract_dir: Option<List<String>>,

        /// A list of hash extractions for each URL.
        pub hash: Option<List<HashExtraction>>,

        /// Instructions for installing the app.
        pub installer: Option<Installer>,

        /// A list of shortcuts to add to the Start Menu.
        pub shortcuts: Option<Vec<Shortcut>>,

        /// A list of template URLs to download.
        /// If a URL contains a fragment starting with '/', the download is renamed,
        /// i.e., https://example.test/app.exe#/app.zip -> app.zip
        pub url: Option<List<String>>,
    }
}

json_struct! {
    /// A subset of an app's manifest that can be automatically updated.
    pub struct Autoupdate {
        /// The app license, either a string or a map.
        pub license: Option<License>,

        /// A map of architectures to their specific manifest.
        pub architecture: Option<HashMap<Arch, AutoupdateArch>>,

        /// A list of messages to show after installation.
        pub notes: Option<List<String>>,

        /// A list of files or directories to persist across upgrades.
        pub persist: Option<Persist>,

        /// If specified, the app is installed as a PowerShell module.
        pub psmodule: Option<PSModule>,

        /// Shared fields.
        #[serde(flatten)]
        pub common: AutoupdateArch,
    }
}

json_struct! {
    /// A subset of an app's manifest that can be customized per architecture.
    pub struct ManifestArch {
        /// A list of executables to add to the user's path.
        pub bin: Option<List<Bin>>,

        /// A regular expression or JsonPath to extract the app's version from the app's URL.
        pub checkver: Option<Checkver>,

        /// A list of directories to add to the user's path, relative to the install directory.
        pub env_add_path: Option<List<String>>,

        /// Set environment variables for the user.
        pub env_set: Option<HashMap<String, String>>,

        /// If an archive is downloaded from the URL, extract a specific directory.
        pub extract_dir: Option<List<String>>,

        /// A list of hashes for each URL.
        /// SHA256, SHA512, SHA1, and MD5 are supported, defaulting to SHA256.
        /// Prefix the hash with 'algo:' to specify the algorithm, i.e., 'sha256:...', 'sha512:...', etc.
        pub hash: Option<List<String>>,

        /// Instructions for installing the app.
        pub installer: Option<Installer>,

        /// A PowerShell script to run before installation.
        pub pre_install: Option<List<String>>,

        /// A PowerShell script to run after installation.
        pub post_install: Option<List<String>>,

        /// A PowerShell script to run before uninstallation.
        pub pre_uninstall: Option<List<String>>,

        /// A PowerShell script to run after uninstallation.
        pub post_uninstall: Option<List<String>>,

        /// A list of shortcuts to add to the Start Menu.
        pub shortcuts: Option<Vec<Shortcut>>,

        /// Instructions for uninstalling the app.
        pub uninstaller: Option<Installer>,

        /// A list of URLs to download.
        /// If a URL contains a fragment starting with '/', the download is renamed,
        /// i.e., https://example.test/app.exe#/app.zip -> app.zip
        pub url: Option<List<String>>,
    }
}

json_struct! {
    /// An app manifest, containing its metadata and installation instructions.
    ///
    /// For specifics, see https://github.com/ScoopInstaller/Scoop/wiki/App-Manifests
    pub struct Manifest {
        /// The app version.
        pub version: String,

        /// The app description.
        pub description: Option<String>,

        /// The app homepage.
        pub homepage: String,

        /// The app license, either a string or a map.
        pub license: License,

        /// A map of architectures to their specific manifest.
        pub architecture: Option<HashMap<Arch, ManifestArch>>,

        /// A template for updating the manifest automatically.
        pub autoupdate: Option<Autoupdate>,

        /// A list of runtime dependencies on other apps.
        pub depends: Option<List<String>>,

        /// Whether or not the installer uses InnoSetup.
        pub innosetup: Option<bool>,

        /// A list of messages to show after installation.
        pub notes: Option<List<String>>,

        /// A list of files or directories to persist across upgrades.
        pub persist: Option<Persist>,

        /// If specified, the app is installed as a PowerShell module.
        pub psmodule: Option<PSModule>,

        /// A map of the app's extra features to a list of optional dependencies.
        /// These will be shown to the user if they not installed yet.
        pub suggest: Option<HashMap<String, List<String>>>,

        /// Common fields.
        #[serde(flatten)]
        pub common: ManifestArch,
    }
}

#[cfg(test)]
mod tests {
    use serde_json;

    use super::*;

    #[test]
    fn deserialize_shortcut() {
        let from_str = serde_json::from_str::<Shortcut>;

        let shortcut = from_str(
            r#"
            ["program.exe", "shortcut_to_program", "--params --to --program", "icon.ico"]
            "#,
        )
        .unwrap();

        assert_eq!(shortcut.executable, "program.exe");
        assert_eq!(shortcut.name, "shortcut_to_program");
        assert_eq!(shortcut.arguments.unwrap(), "--params --to --program");
        assert_eq!(shortcut.icon.unwrap(), "icon.ico");

        assert!(from_str("[]").is_err());
        assert!(from_str(r#"["foo"]"#).is_err());
        assert!(from_str(r#"["foo", "bar"]"#).is_ok());
    }

    #[test]
    fn serialize_shortcut() {
        let to_string = serde_json::to_string::<Shortcut>;

        let shortcut = Shortcut {
            executable: "shovel.exe".to_owned(),
            name: "Shovel".to_owned(),
            ..Default::default()
        };

        assert_eq!(to_string(&shortcut).unwrap(), r#"["shovel.exe","Shovel"]"#);

        let shortcut_with_params = Shortcut {
            executable: "gitk.exe".to_owned(),
            name: "Gitk".to_owned(),
            arguments: Some("--all".to_owned()),
            ..Default::default()
        };

        assert_eq!(
            to_string(&shortcut_with_params).unwrap(),
            r#"["gitk.exe","Gitk","--all"]"#
        );

        let shortcut_with_icon = Shortcut {
            executable: "foo".to_owned(),
            name: "bar".to_owned(),
            icon: Some("baz".to_owned()),
            ..Default::default()
        };

        assert_eq!(
            to_string(&shortcut_with_icon).unwrap(),
            r#"["foo","bar","","baz"]"#
        );
    }

    #[test]
    fn deserialize_shim() {
        let from_str = serde_json::from_str::<Shim>;

        let shim = from_str(
            r#"
            ["shovel.exe", "shv", "--config", "~\\scoop\\persist\\shovel\\config.json"]
            "#,
        )
        .unwrap();

        assert_eq!(shim.executable, "shovel.exe");
        assert_eq!(shim.name, "shv");
        assert_eq!(
            shim.arguments,
            vec!["--config", r"~\scoop\persist\shovel\config.json"]
        );

        assert!(from_str("[]").is_err());
        assert!(from_str(r#"["foo"]"#).is_err());
        assert!(from_str(r#"["foo", "bar"]"#).is_ok());
    }

    #[test]
    fn serialize_shim() {
        let to_string = serde_json::to_string::<Shim>;

        let shim = Shim {
            executable: "git.exe".to_owned(),
            name: "g".to_owned(),
            ..Default::default()
        };

        assert_eq!(to_string(&shim).unwrap(), r#"["git.exe","g"]"#);

        let shim_with_args = Shim {
            executable: "helix.exe".to_owned(),
            name: "hx".to_owned(),
            arguments: vec!["--config", r"~\scoop\persist\helix\config.toml"]
                .into_iter()
                .map(|s| s.to_owned())
                .collect(),
        };

        assert_eq!(
            to_string(&shim_with_args).unwrap(),
            r#"["helix.exe","hx","--config","~\\scoop\\persist\\helix\\config.toml"]"#
        )
    }
}