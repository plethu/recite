//! Native Unix bytes stay reversible at the bounded socket boundary.
use crate::startup::EditorCommand;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::PathBuf,
};

pub(crate) mod optional_path {
    use super::*;
    pub fn serialize<S: Serializer>(
        path: &Option<PathBuf>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        path.as_ref()
            .map(|path| path.as_os_str().as_bytes())
            .serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<PathBuf>, D::Error> {
        let bytes = Option::<Vec<u8>>::deserialize(deserializer)?;
        Ok(bytes.map(|bytes| PathBuf::from(OsString::from_vec(bytes))))
    }
}
#[derive(Serialize, Deserialize)]
struct Command {
    program: Vec<u8>,
    args: Vec<Vec<u8>>,
}
pub(crate) mod optional_editor {
    use super::*;
    pub fn serialize<S: Serializer>(
        command: &Option<EditorCommand>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        command
            .as_ref()
            .map(|command| Command {
                program: command.program.as_bytes().to_vec(),
                args: command
                    .args
                    .iter()
                    .map(|arg| arg.as_bytes().to_vec())
                    .collect(),
            })
            .serialize(serializer)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<EditorCommand>, D::Error> {
        Ok(
            Option::<Command>::deserialize(deserializer)?.map(|command| EditorCommand {
                program: OsString::from_vec(command.program),
                args: command.args.into_iter().map(OsString::from_vec).collect(),
            }),
        )
    }
}
