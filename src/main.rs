// A nextcloud server path to test against
const SERVER_URL: &str = "https://testcloud.chaos/remote.php/dav/files/test/";

use rustydav::client::Client;


mod errors;
mod prop;
mod webdav;
mod filesystem;

fn main() {
    // Webdav client setup
    let webdav_client = Client::init("test", "test");

    let props = webdav::list(
        &webdav_client,
        SERVER_URL,
        webdav::PropfindDepth::Recursive,
    ).unwrap();

    println!("{:#?}", props);
}
