use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditOp {
    Write {
        path: PathBuf,
        contents: String,
    },
    StrReplace {
        path: PathBuf,
        old_string: String,
        new_string: String,
    },
}

impl EditOp {
    pub fn path(&self) -> &PathBuf {
        match self {
            EditOp::Write { path, .. } | EditOp::StrReplace { path, .. } => path,
        }
    }
}
