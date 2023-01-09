const SERVER_URL: &str = "https://testcloud.chaos/remote.php/dav/files/test/";

use std::path::Path;
use std::path::PathBuf;

use rustydav::client;
use rustydav::prelude::*;

use roxmltree;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ResourceType {
    /// Regular file
    File,
    /// Folder
    Collection,
}

#[derive(Debug)]
pub struct Prop {
    etag: String,
    path: PathBuf,
    size: u64,
    resource_type: ResourceType,
}

impl Prop {
    pub fn new(etag: String, path: PathBuf, size: u64, resource_type: ResourceType) -> Self {
        Prop {
            etag,
            path,
            size,
            resource_type,
        }
    }

    pub fn etag(&self) -> &str {
        &self.etag
    }

    pub fn path(&self) -> &Path {
        &self.path.as_path()
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn resource_type(&self) -> ResourceType {
        self.resource_type
    }
}

impl From<roxmltree::Node<'_, '_>> for Prop {
    fn from(_: roxmltree::Node<'_, '_>) -> Self {
        todo!()
    }
}

fn main() {
    let webdav_client = client::Client::init("test", "test");
    let res = webdav_client.list(SERVER_URL, "1").unwrap();

    let binding = res.text().unwrap();
    let parser = roxmltree::Document::parse(&binding).unwrap();

    println!("{:#?}", parser.root().descendants().count());

    //for element in parser.descendants() {
    //    println!("Tag name.name(): {:#?}", element.tag_name());
    //}

    let first_prop = parser
        .descendants()
        .find(|n| n.has_tag_name("prop"))
        .unwrap();
    println!("{:#?}", first_prop);
    println!("{:?}", first_prop.descendants().count());

    for el in first_prop.children() {
        println!("{:#?}", el.tag_name());
    }
}
