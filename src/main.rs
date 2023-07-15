// A nextcloud server path to test against
const SERVER_URL: &str = "https://testcloud.chaos/remote.php/dav/files/test";

use fuser::{self, MountOption};
use rustydav::client::Client;

#[macro_use]
extern crate log;

mod errors;
mod filesystem;
mod prop;
mod webdav;

fn main() {
    env_logger::init();
    // Webdav client setup
    let webdav_client = Client::init("test", "test");
    let webdav_drive = webdav::WebdavDrive::new(SERVER_URL.to_string(), webdav_client);

    let props = webdav_drive
        .list("/", webdav::PropfindDepth::Recursive)
        .unwrap();

    let fs = filesystem::FuseFilesystem::init(webdav_drive);

    let mut mount_options = vec![MountOption::NoAtime];
    // read only for now
    mount_options.push(MountOption::RO);

    println!("{:#?}", props);

    let _mount = fuser::mount2(fs, "/home/nick/repo/fuse/webdav_fuse/mnt", &mount_options);
}
