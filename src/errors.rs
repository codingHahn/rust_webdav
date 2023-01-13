#[derive(Debug, Clone)]
pub enum Errors {
    /// The reqeust from the server errored out
    WebDavReqeustFailed,
    /// The size of a prop that was returned is nonsense
    PropSizeError,
    /// The xml cannot be parsed. This happens when a response is malformed
    XMLDocumentParseError(roxmltree::Error),
    /// The XML tag did not contain any text when it should have. Contains the tag name
    XMLTagEmptyWhenItShouldNot(String),
    /// The timestamp could not be converted to UNIX time
    DateTimeConversionError(chrono::ParseError),
}
