use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEmpty, ReplyEntry,
    ReplyOpen, Request, FUSE_ROOT_ID,
};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    time::Duration,
    time::UNIX_EPOCH,
};

use libc;

use crate::{
    errors::Errors,
    prop::{Prop, ResourceType},
    webdav::{PropfindDepth, WebdavDrive},
};

const TTL: std::time::Duration = Duration::from_secs(5);

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct InodeId(u64);

impl InodeId {
    fn is_filesystem_root(&self) -> bool {
        return self.0 == FUSE_ROOT_ID;
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub struct FileHandleId(u64);

/// Contains all states a file can be in
#[derive(Debug)]
pub enum FileState {
    /// File is downloaded and (to our knowledge) up to date
    Local,
    /// File is not downloaded and available from remote
    RemoteOnly,
    /// File is downloaded, but changed locally
    ChangedLocally,
    /// File is downloaded, but changed remote
    ChangedRemote,
    /// Some type of conflict
    Conflict,
    Downloading,
    Uploading,
}

#[derive(Debug)]
pub struct FileAttributes {
    name: OsString,
    size: u64,
    mtime: u64,
    is_directory: bool,
    state: FileState,
}

impl FileAttributes {
    pub fn fuser_filetype(&self) -> FileType {
        if self.is_directory {
            FileType::Directory
        } else {
            FileType::RegularFile
        }
    }
}

impl From<Prop> for File {
    /// Converts a Prop to a file.
    /// Only stores the file_name of the prop's path
    fn from(value: Prop) -> Self {
        let is_folder = match value.resource_type() {
            ResourceType::File => false,
            ResourceType::Collection => true,
            ResourceType::Invalid => {
                panic!("Tried to convert prop with ResourceType 'Invalid' to Inode")
            }
        };
        Self {
            attr: FileAttributes {
                name: value
                    .path()
                    .file_name()
                    .expect("File Name ended in '..'")
                    .into(),
                size: value.size(),
                mtime: value.last_modified(),
                is_directory: is_folder,
                state: FileState::RemoteOnly,
            },
            etag: value.etag().to_string(),
        }
    }
}

/// Maps file paths to inodes and represents the tree structure
pub struct Inode {
    children: BTreeMap<OsString, InodeId>,
    parent: InodeId,
}

impl Inode {
    pub fn new(parent: InodeId) -> Self {
        Self {
            children: BTreeMap::new(),
            parent,
        }
    }

    pub fn add_child(&mut self, name: OsString, inode: InodeId) {
        self.children.insert(name, inode);
    }
}

#[derive(Debug)]
pub struct File {
    attr: FileAttributes,
    etag: String,
}

impl File {
    fn init_root() -> Self {
        let root_inode = File {
            attr: FileAttributes {
                name: "/".to_string().into(),
                size: 0,
                mtime: 0,
                is_directory: true,
                state: FileState::Local,
            },
            etag: "root".to_string(),
        };
        return root_inode;
    }

    pub fn attributes(&self) -> &FileAttributes {
        &self.attr
    }

    /// Transforms the FileAttributes of an inode into the libfuse-native FileAttr
    pub fn to_file_attr(&self, inode: InodeId) -> FileAttr {
        use std::os::unix::fs::MetadataExt;
        let uid = std::fs::metadata("/proc/self").map(|m| m.uid()).unwrap();
        let gid = std::fs::metadata("/proc/self").map(|m| m.gid()).unwrap();
        let attr = &self.attr;
        let ft = attr.fuser_filetype();

        FileAttr {
            ino: inode.0,
            size: attr.size,
            blocks: attr.size / 4096,
            atime: UNIX_EPOCH + Duration::from_secs(attr.mtime),
            mtime: UNIX_EPOCH + Duration::from_secs(attr.mtime),
            ctime: UNIX_EPOCH + Duration::from_secs(attr.mtime),
            crtime: UNIX_EPOCH + Duration::from_secs(attr.mtime),
            kind: ft,
            perm: 0o77,
            nlink: 0,
            uid,
            gid,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }
}

pub struct FuseFilesystem {
    inodes: BTreeMap<InodeId, Inode>,
    files: BTreeMap<InodeId, File>,
    next_inode: InodeId,
    next_fd: FileHandleId,
    drive: WebdavDrive,
}

impl FuseFilesystem {
    fn new(drive: WebdavDrive) -> Self {
        return Self {
            inodes: BTreeMap::new(),
            files: BTreeMap::new(),
            next_inode: InodeId(2),
            next_fd: FileHandleId(2),
            drive,
        };
    }

    /// Initializes a filesystem with an root node
    pub fn init(drive: WebdavDrive) -> Self {
        let mut fs = Self::new(drive);
        let root_inode = Inode::new(InodeId(FUSE_ROOT_ID));
        let root_file = File::init_root();

        fs.inodes.insert(InodeId(FUSE_ROOT_ID), root_inode);
        fs.files.insert(InodeId(FUSE_ROOT_ID), root_file);
        fs
    }

    /// Returns next `InodeId` and increments `self.next_inode`
    fn next_inode(&mut self) -> InodeId {
        let ino = self.next_inode;
        self.next_inode = InodeId(ino.0 + 1);
        ino
    }

    /// Gathers information about an inode by parent inode and name
    fn lookup_(&self, parent: InodeId, name_of_file: &OsStr) -> Result<FileAttr, Errors> {
        let mut parent_inode = self
            .inodes
            .get(&parent)
            .ok_or(Errors::ParentInodeNotFound(parent))?;
        if parent_inode.children.is_empty() {
            //self.readdir(parent, 0)?;
            parent_inode = self
                .inodes
                .get(&parent)
                .ok_or(Errors::ParentInodeNotFound(parent))?
                .clone();
        }
        let inode = parent_inode
            .children
            .get(name_of_file)
            .ok_or(Errors::FileDoesNotExist(name_of_file.into()))?;
        let file = self
            .files
            .get(inode)
            .ok_or(Errors::ChildInodeNotFound(*inode))?;
        Ok(file.to_file_attr(*inode))
    }

    fn readdir2(
        &mut self,
        inode: InodeId,
        offset: usize,
    ) -> Result<Vec<(InodeId, FileType, OsString)>, Errors> {
        let mut result = Vec::new();
        let ino = self
            .inodes
            .get(&inode)
            .ok_or(Errors::InodeNotFound(inode))?;

        if offset == 0 {
            result.push((ino.parent, FileType::Directory, "..".into()));
            result.push((inode, FileType::Directory, ".".into()));
        }

        let full_path = self.full_path_of_inode(&inode)?;

        let props = self.drive.list(&full_path, PropfindDepth::WithChildren)?;

        let _files: Vec<File> = props.into_iter().map(|f| f.into()).skip(offset).collect();

        println!("Returned children of {}: \n {:#?}", full_path, _files);

        for f in _files {
            result.push((
                self.next_inode(),
                f.attributes().fuser_filetype(),
                f.attributes().name.clone(),
            ))
        }

        //TODO: Actually show files and not just . and ..

        Ok(result)
    }

    fn getattributes(&self, inode: InodeId) -> Result<FileAttr, Errors> {
        let file_attr = self.files.get(&inode).ok_or(Errors::InodeNotFound(inode))?;
        Ok(file_attr.to_file_attr(inode))
    }

    /// recursive function that builds an filesystem-absolute path by traversing the inode tree
    /// upwards
    fn _full_path_of_inode(&self, inode: &InodeId) -> Result<Vec<String>, Errors> {
        let name = self
            .files
            .get(inode)
            .ok_or(Errors::FileEntryMissing(*inode))?
            .attr
            .name
            .clone();
        let mut path: Vec<String> = vec![name.into_string().map_err(Errors::NonUnicodeInPath)?];

        let parent_inode = self
            .inodes
            .get(inode)
            .ok_or(Errors::ParentInodeNotFound(*inode))?
            .parent;

        // Recurse until root node is reached, then return
        while !parent_inode.is_filesystem_root() {
            path.append(&mut self._full_path_of_inode(&parent_inode)?);
        }

        Ok(path)
    }

    /// Returns the filesystem-absolute path of an inode
    fn full_path_of_inode(&self, inode: &InodeId) -> Result<String, Errors> {
        let mut path_vec = self._full_path_of_inode(inode)?;
        path_vec.reverse();
        Ok(path_vec.into_iter().collect())
    }
}

impl Filesystem for FuseFilesystem {
    fn readdir(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        let files_in_dir = self
            .readdir2(InodeId(ino), offset.try_into().unwrap())
            .unwrap();
        for (idx, entry) in files_in_dir.iter().enumerate() {
            let full = reply.add(entry.0 .0, idx.try_into().unwrap(), entry.1, &entry.2);
            if full {
                break;
            }
        }
        reply.ok();
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, reply: ReplyAttr) {
        let attr = self.getattributes(InodeId(ino));
        reply.attr(&TTL, &attr.unwrap());
    }

    fn lookup(
        &mut self,
        _req: &Request<'_>,
        parent: u64,
        name: &std::ffi::OsStr,
        reply: ReplyEntry,
    ) {
        if let Ok(attr) = self.lookup_(InodeId(parent), name) {
            reply.entry(&TTL, &attr, 0);
        } else {
            reply.error(libc::ENOENT);
        }
    }
}
