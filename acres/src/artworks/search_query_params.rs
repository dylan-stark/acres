use serde::Serialize;
use serde::ser::SerializeSeq;

#[derive(Debug)]
pub(super) struct SearchQueryParams {
    pub(super) q: Option<String>,
}

impl Serialize for SearchQueryParams {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        if let Some(q) = &self.q {
            seq.serialize_element(&("q", q))?
        }
        seq.end()
    }
}
