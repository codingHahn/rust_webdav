use crate::prop::{Prop, ResourceType};

/// Contains all states a file can be in
pub enum FileState {
    Local,
    RemoteOnly,
    ChangedLocally,
    ChangedRemote,
    Conflict,
    Downloading,
    Uploading,
}

pub struct InodeAttr {
    size: u64,
    mtime: i64,
    is_directory: bool,
    state: FileState,
}

impl From<Prop> for InodeAttr {
    fn from(value: Prop) -> Self {
        let is_folder = match value.resource_type() {
            ResourceType::File => false,
            ResourceType::Collection => true,
            ResourceType::Invalid => {
                panic!("Tried to convert prop with ResourceType 'Invalid' to Inode")
            }
        };
        Self {
            size: value.size(),
            mtime: value.last_modified(),
            is_directory: is_folder,
            state: FileState::RemoteOnly,
        }
    }
}
