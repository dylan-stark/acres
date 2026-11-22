//! `Serialize`/`Deserialize` through each type's IIIF string form, so serialized values look like
//! the URLs and parameters they represent, and deserializing validates exactly as `FromStr` does.

use std::{fmt, marker::PhantomData, str::FromStr};

use serde::{Deserializer, Serializer, de};

use crate::{
    Degree, Format, ImageRequest, InformationRequest, Percentage, Quality, Region, Rotation, Size,
    Uri,
};

struct FromStrVisitor<T>(PhantomData<T>);

impl<T> de::Visitor<'_> for FromStrVisitor<T>
where
    T: FromStr,
    T::Err: fmt::Display,
{
    type Value = T;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "a string")
    }

    fn visit_str<E: de::Error>(self, v: &str) -> Result<T, E> {
        v.parse().map_err(E::custom)
    }
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
    Degree,
    Format,
    ImageRequest,
    InformationRequest,
    Percentage,
    Quality,
    Region,
    Rotation,
    Size,
    Uri,
);

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use rstest::rstest;
    use serde::{Serialize, de::DeserializeOwned};

    use crate::{
        Degree, Format, ImageRequest, InformationRequest, Percentage, Quality, Region, Rotation,
        Size, Uri,
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
        round_trips::<Uri>(s);
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
        round_trips::<Degree>("33.33");
        round_trips::<Region>("0,0,10,10");
        round_trips::<Size>("pct:50");
        round_trips::<Rotation>("!180");
        round_trips::<Quality>("bitonal");
        round_trips::<Format>("webp");
    }

    #[test]
    fn uri_serializes_as_string() {
        let uri: Uri = "https://example.org/images/12345".parse().unwrap();
        assert_eq!(
            serde_json::to_string(&uri).unwrap(),
            r#""https://example.org/images/12345""#
        );
    }

    #[rstest]
    #[case::bad_percentage(r#""banana""#, "unable to parse percentage")]
    #[case::out_of_range(r#""110""#, "invalid percentage: 110")]
    fn percentage_deserialize_validates(#[case] json: &str, #[case] expected: &str) {
        let err = serde_json::from_str::<Percentage>(json).unwrap_err();
        assert!(err.to_string().contains(expected), "{err}");
    }

    #[rstest]
    #[case::dot_identifier(r#""https://example.org/images/..""#)]
    #[case::query(r#""https://example.org/images/12345?x=1""#)]
    #[case::not_a_string(r#"{"scheme":"Https"}"#)]
    fn uri_deserialize_validates(#[case] json: &str) {
        assert!(serde_json::from_str::<Uri>(json).is_err());
    }
}
