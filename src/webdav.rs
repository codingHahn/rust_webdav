use crate::errors::Errors;
use crate::prop::*;
use chrono::prelude::*;
use rustydav::client;

/// PROPFIND supports three different depths:
///     - ELEMENT_ONLY, which corresponds to "0" and returns information about
///       the requested prop only.
///     - WITH_CHILDREN, which corresponds to "1" and also returns information
///       about children of the prop, if it has any.
///     - RECURSIVE, which corresponds to "infinity" and recursively returns
///       information about the whole tree down.
#[derive(Debug, Clone, Copy)]
pub enum PropfindDepth {
    ElementOnly,
    WithChildren,
    Recursive,
}

impl From<PropfindDepth> for &str {
    fn from(depth: PropfindDepth) -> &'static str {
        match depth {
            PropfindDepth::ElementOnly => "0",
            PropfindDepth::WithChildren => "1",
            PropfindDepth::Recursive => "infinity",
        }
    }
}

/// Information about the drive
pub struct WebdavDrive {
    /// Prefix of the URL to prepend on request
    prefix: String,
    client: client::Client,
}

impl WebdavDrive {
    pub fn new(prefix: String, client: client::Client) -> Self {
        Self { prefix, client }
    }
    /// Executes a "PROPFIND" request against `path` with depth as specified in `PropfindDepth`
    pub fn list(&self, path: &str, depth: PropfindDepth) -> Result<Vec<Prop>, Errors> {
        let mut ret: Vec<Prop> = vec![];

        let http_response = self
            .client
            .list(&(self.prefix.clone() + path), depth.into())
            .map_err(|_| Errors::WebDavReqeustFailed)?;
        let resp_text = http_response
            .text()
            .map_err(|_| Errors::WebDavReqeustFailed)?;
        let parser =
            roxmltree::Document::parse(&resp_text).map_err(Errors::XMLDocumentParseError)?;

        // Gets all nodes with "response" tag. One prop per response
        let responses = parser.descendants().filter(|n| n.has_tag_name("response"));

        for response in responses {
            // Get the first Prop returned (file or collection)
            let props = response
                .descendants()
                .find(|n| n.has_tag_name("prop"))
                .ok_or_else(|| Errors::XMLTagEmptyWhenItShouldNot("prop".into()))?;

            // the href, which contains the path (I think?) is one level above the prop
            let href = response
                .descendants()
                .find(|n| n.has_tag_name("href"))
                .ok_or_else(|| Errors::XMLTagEmptyWhenItShouldNot("href".into()))?
                .text()
                .ok_or_else(|| Errors::XMLTagEmptyWhenItShouldNot("href".into()))?;

            //println!("{:#?}", props);
            //println!("{:?}", props.descendants().count());

            let mut propb = PropBuilder::new().path(href.into());

            // Iterate over all elements of the prop node. This extracts important file metadata
            // such as the etag, last-modified-time, resource_type and the size
            for el in props.children() {
                // Handle the current tag accordingly
                match el.tag_name().name() {
                    "getlastmodified" => {
                        propb = propb.last_modified(
                            DateTime::parse_from_rfc2822(el.text().ok_or_else(|| {
                                Errors::XMLTagEmptyWhenItShouldNot("getlastmodified".into())
                            })?)
                            .map_err(Errors::DateTimeConversionError)?
                            .timestamp()
                            .try_into()
                            .unwrap_or_default(),
                        );
                    }
                    "resourcetype" => {
                        let restype = el
                            .has_children()
                            .then_some(())
                            .map_or(ResourceType::File, |_| ResourceType::Collection);
                        propb = propb.resource_type(restype)
                    }
                    "getcontentlength" => {
                        propb = propb.size(
                            el.text()
                                .ok_or_else(|| {
                                    Errors::XMLTagEmptyWhenItShouldNot("getcontentlength".into())
                                })?
                                .parse::<u64>()
                                .map_err(|_| Errors::PropSizeError)?,
                        )
                    }
                    "getetag" => {
                        propb = propb.etag(
                            el.text()
                                .ok_or_else(|| {
                                    Errors::XMLTagEmptyWhenItShouldNot("getetag".into())
                                })?
                                .replace('\"', "")
                                .to_string(),
                        )
                    }
                    unknown_tag => println!("unhandled tag name found in xml: {unknown_tag}"),
                }
            }
            ret.push(propb.build())
        }
        Ok(ret)
    }
}
