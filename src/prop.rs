use std::path::Path;
use std::path::PathBuf;

/// A Prop has a type. Implemented are `Files` and `Collection`, the latter
/// are equivalent to folders.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ResourceType {
    /// Regular file
    File,
    /// Folder
    Collection,
    /// Initial Value
    Invalid,
}

/// Stores the data belonging to what WebDAV calls a "Prop".
/// This can be a file or a collection (basically a folder)
#[derive(Debug)]
pub struct Prop {
    /// Etag is guaranteed to be stable if the Prop has not changed
    etag: String,
    /// Path of the prop
    path: PathBuf,
    /// Size in bytes
    size: u64,
    /// Unix timestamp of the last modification date
    last_modified: i64,
    /// Type of the prop
    resource_type: ResourceType,
}

impl Prop {
    pub fn new(
        etag: String,
        path: PathBuf,
        size: u64,
        resource_type: ResourceType,
        last_modified: i64,
    ) -> Self {
        Prop {
            etag,
            path,
            size,
            last_modified,
            resource_type,
        }
    }

    // Getters

    pub fn etag(&self) -> &str {
        &self.etag
    }

    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn last_modified(&self) -> i64 {
        self.last_modified
    }

    pub fn resource_type(&self) -> ResourceType {
        self.resource_type
    }
}

/// Builder for `Prop`
#[derive(Debug)]
pub struct PropBuilder {
    prop: Prop,
}

impl PropBuilder {
    pub fn new() -> Self {
        Self {
            prop: Prop {
                etag: "".to_string(),
                path: "".into(),
                size: 0,
                last_modified: -1,
                resource_type: ResourceType::Invalid,
            },
        }
    }
    pub fn etag(mut self, etag: String) -> Self {
        self.prop.etag = etag;
        self
    }

    pub fn path(mut self, path: PathBuf) -> Self {
        self.prop.path = path;
        self
    }

    pub fn size(mut self, size: u64) -> Self {
        self.prop.size = size;
        self
    }

    pub fn last_modified(mut self, last_modified: i64) -> Self {
        self.prop.last_modified = last_modified;
        self
    }

    pub fn resource_type(mut self, resource_type: ResourceType) -> Self {
        self.prop.resource_type = resource_type;
        self
    }

    pub fn build(self) -> Prop {
        self.prop
    }
}
