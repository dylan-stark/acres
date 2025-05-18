use serde::Serialize;
use serde::ser::SerializeSeq;

#[derive(Debug)]
pub(super) struct CollectionQueryParams {
    pub(super) ids: Option<Vec<u32>>,
    pub(super) limit: Option<u32>,
    pub(super) page: Option<u32>,
    pub(super) fields: Vec<String>,
    pub(super) include: Vec<String>,
}

impl Serialize for CollectionQueryParams {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        if let Some(ids) = &self.ids {
            let ids_string = ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",");
            seq.serialize_element(&("ids", ids_string))?
        }
        if let Some(limit) = &self.limit {
            seq.serialize_element(&("limit", limit))?
        }
        if let Some(page) = &self.page {
            seq.serialize_element(&("page", page))?
        }
        if !self.fields.is_empty() {
            seq.serialize_element(&("fields", self.fields.join(",")))?;
        }
        if !self.include.is_empty() {
            seq.serialize_element(&("include", self.include.join(",")))?;
        }
        seq.end()
    }
}
