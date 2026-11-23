//! `Serialize`/`Deserialize` through each type's IIIF string form, so serialized values look like
//! the URLs and parameters they represent, and deserializing validates exactly as `FromStr` does.

use std::{error::Error, fmt, marker::PhantomData, str::FromStr};

use serde::{Deserializer, Serializer, de};

use crate::{
    BaseUri, Degrees, Format, ImageRequest, InformationRequest, Percentage, Quality, Region,
    Rotation, Size,
};

struct FromStrVisitor<T>(PhantomData<T>);

impl<T> de::Visitor<'_> for FromStrVisitor<T>
where
    T: FromStr,
    T::Err: Error,
{
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a string")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<T, E> {
        v.parse().map_err(|e: T::Err| E::custom(chain(&e)))
    }
}

/// `e` and its sources joined with `: `. Serde keeps only an error's message, and our wrapping
/// errors leave their source out of it, so without this a request's message would lose the cause.
fn chain(e: &dyn Error) -> String {
    let mut message = e.to_string();
    let mut source = e.source();
    while let Some(e) = source {
        message.push_str(": ");
        message.push_str(&e.to_string());
        source = e.source();
    }
    message
}

macro_rules! serde_via_str {
    ($($ty:ty),* $(,)?) => {$(
        impl serde::Serialize for $ty {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(self)
            }
        }

        impl<'de> serde::Deserialize<'de> for $ty {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserializer.deserialize_str(FromStrVisitor(PhantomData))
            }
        }
    )*};
}

serde_via_str!(
    Degrees,
    Format,
    ImageRequest,
    InformationRequest,
    Percentage,
    Quality,
    Region,
    Rotation,
    Size,
    BaseUri,
);

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;
    use serde::{Serialize, de::DeserializeOwned};

    use crate::{
        BaseUri, Degrees, Format, ImageRequest, InformationRequest, Percentage, Quality, Region,
        Rotation, Size,
    };

    fn round_trips<T>(s: &str)
    where
        T: Serialize + DeserializeOwned + PartialEq + Debug + std::str::FromStr,
        T::Err: Debug,
    {
        let json = serde_json::to_string(&s).unwrap();
        let value: T = serde_json::from_str(&json).unwrap();
        assert_eq!(value, s.parse::<T>().unwrap());
        assert_eq!(serde_json::to_string(&value).unwrap(), json);
    }

    #[rstest]
    #[case("https://example.org/images/12345")]
    #[case("http://127.0.0.1:5555/some/path/to/a%20b")]
    fn uri_round_trips(#[case] s: &str) {
        round_trips::<BaseUri>(s);
    }

    #[test]
    fn requests_round_trip() {
        round_trips::<ImageRequest>(
            "https://example.org/images/12345/pct:10,20,30,40/!640,480/!90/gray.png",
        );
        round_trips::<InformationRequest>("https://example.org/images/12345/info.json");
    }

    #[test]
    fn parameters_round_trip() {
        round_trips::<Percentage>("4.2");
        round_trips::<Degrees>("33.33");
        round_trips::<Region>("0,0,10,10");
        round_trips::<Size>("pct:50");
        round_trips::<Rotation>("!180");
        round_trips::<Quality>("bitonal");
        round_trips::<Format>("webp");
    }

    #[test]
    fn uri_serializes_as_string() {
        let uri: BaseUri = "https://example.org/images/12345".parse().unwrap();
        assert_eq!(
            serde_json::to_string(&uri).unwrap(),
            r#""https://example.org/images/12345""#
        );
    }

    #[rstest]
    #[case::bad_percentage(r#""banana""#, "invalid percentage: banana: invalid float literal")]
    #[case::out_of_range(r#""110""#, "percentage out of range: 110")]
    fn percentage_deserialize_validates(#[case] json: &str, #[case] expected: &str) {
        let err = serde_json::from_str::<Percentage>(json).unwrap_err();
        assert!(err.to_string().contains(expected), "{err}");
    }

    #[test]
    fn request_deserialize_error_includes_cause() {
        let err = serde_json::from_str::<ImageRequest>(
            r#""https://example.org/images/12345/full/max/0/sepia.jpg""#,
        )
        .unwrap_err();
        assert!(
            err.to_string()
                .contains("invalid parameter in image request: invalid quality: sepia"),
            "{err}"
        );
    }

    #[rstest]
    #[case::dot_identifier(r#""https://example.org/images/..""#)]
    #[case::query(r#""https://example.org/images/12345?x=1""#)]
    #[case::not_a_string(r#"{"scheme":"Https"}"#)]
    fn uri_deserialize_validates(#[case] json: &str) {
        assert!(serde_json::from_str::<BaseUri>(json).is_err());
    }
}
