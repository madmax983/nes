import re

with open('crates/nes-ai/src/error.rs', 'r') as f:
    content = f.read()

content = content.replace(
    '#[derive(Debug, Error)]\npub enum AiError {',
    '/// Error types emitted by the AI training and evaluation pipeline.\n#[derive(Debug, Error)]\npub enum AiError {'
)

content = content.replace(
    '    #[error("unsupported operation: {0}")]\n    Unsupported(&\'static str),',
    '    /// An operation was attempted that is not supported by the current environment or profile.\n    #[error("unsupported operation: {0}")]\n    Unsupported(&\'static str),'
)

content = content.replace(
    '    #[error("failed to create snapshot directory \'{path}\': {source}")]\n    SnapshotDirCreate {',
    '    /// Failed to create the directory hierarchy for storing environment snapshots.\n    #[error("failed to create snapshot directory \'{path}\': {source}")]\n    SnapshotDirCreate {'
)
content = content.replace(
    '        path: PathBuf,\n        #[source]\n        source: std::io::Error,\n    },',
    '        /// The path that could not be created.\n        path: PathBuf,\n        /// The underlying IO error.\n        #[source]\n        source: std::io::Error,\n    },',
    1
)


content = content.replace(
    '    #[error("failed to serialize snapshot bundle: {source}")]\n    SnapshotSerialize {',
    '    /// Failed to serialize the snapshot bundle into JSON format.\n    #[error("failed to serialize snapshot bundle: {source}")]\n    SnapshotSerialize {'
)
content = content.replace(
    '        #[source]\n        source: serde_json::Error,\n    },',
    '        /// The underlying serialization error.\n        #[source]\n        source: serde_json::Error,\n    },',
    1
)


content = content.replace(
    '    #[error("failed to write snapshot bundle \'{path}\': {source}")]\n    SnapshotWrite {',
    '    /// Failed to write the serialized snapshot bundle to disk.\n    #[error("failed to write snapshot bundle \'{path}\': {source}")]\n    SnapshotWrite {'
)
content = content.replace(
    '        path: PathBuf,\n        #[source]\n        source: std::io::Error,\n    },',
    '        /// The destination path.\n        path: PathBuf,\n        /// The underlying IO error.\n        #[source]\n        source: std::io::Error,\n    },',
    1
)

content = content.replace(
    '    #[error(\n        "failed to read snapshot bundle \'{path}\': {source}\\nHint: Ensure the file exists or check the profile configuration."\n    )]\n    SnapshotRead {',
    '    /// Failed to read the snapshot bundle from disk.\n    #[error(\n        "failed to read snapshot bundle \'{path}\': {source}\\nHint: Ensure the file exists or check the profile configuration."\n    )]\n    SnapshotRead {'
)
content = content.replace(
    '        path: PathBuf,\n        #[source]\n        source: std::io::Error,\n    },',
    '        /// The path that could not be read.\n        path: PathBuf,\n        /// The underlying IO error.\n        #[source]\n        source: std::io::Error,\n    },',
    1
)

content = content.replace(
    '    #[error("failed to parse snapshot bundle \'{path}\': {source}")]\n    SnapshotParse {',
    '    /// Failed to parse the snapshot bundle JSON data.\n    #[error("failed to parse snapshot bundle \'{path}\': {source}")]\n    SnapshotParse {'
)
content = content.replace(
    '        path: PathBuf,\n        #[source]\n        source: serde_json::Error,\n    },',
    '        /// The path of the corrupted snapshot file.\n        path: PathBuf,\n        /// The underlying JSON parsing error.\n        #[source]\n        source: serde_json::Error,\n    },',
    1
)

content = content.replace(
    '    #[error("unsupported snapshot bundle version: expected {expected}, found {found}")]\n    SnapshotVersionMismatch { expected: u32, found: u32 },',
    '    /// The loaded snapshot has an incompatible version number.\n    #[error("unsupported snapshot bundle version: expected {expected}, found {found}")]\n    SnapshotVersionMismatch { \n        /// The expected version number.\n        expected: u32, \n        /// The version number found in the file.\n        found: u32 \n    },'
)


content = content.replace(
    '    #[error("failed to create artifact directory \'{path}\': {source}")]\n    ArtifactDirCreate {',
    '    /// Failed to create the directory for storing AI model artifacts.\n    #[error("failed to create artifact directory \'{path}\': {source}")]\n    ArtifactDirCreate {'
)
content = content.replace(
    '        path: PathBuf,\n        #[source]\n        source: std::io::Error,\n    },',
    '        /// The path of the directory that failed creation.\n        path: PathBuf,\n        /// The underlying IO error.\n        #[source]\n        source: std::io::Error,\n    },',
    1
)


with open('crates/nes-ai/src/error.rs', 'w') as f:
    f.write(content)
