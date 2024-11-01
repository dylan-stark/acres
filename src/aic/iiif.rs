#![allow(dead_code, clippy::upper_case_acronyms)]
use std::fmt::{self, Debug};

use async_trait::async_trait;
use bytes::Bytes;
use reqwest::header::HeaderMap;

use crate::aic::Result;

enum Format {
    JPG,
    PNG,
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Format::JPG => write!(f, "jpg"),
            Format::PNG => write!(f, "png"),
        }
    }
}

enum Region {
    Full,
    Pixels(usize, usize, usize, usize),
    Percent(f32, f32, f32, f32),
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Region::Full => write!(f, "full"),
            Region::Pixels(x, y, w, h) => write!(f, "{x},{y},{w},{h}"),
            Region::Percent(x, y, w, h) => write!(f, "pct:{x},{y},{w},{h}"),
        }
    }
}

enum Quality {
    Bitonal,
    Color,
    Default,
    Gray,
}

impl fmt::Display for Quality {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Quality::Bitonal => write!(f, "bitonal"),
            Quality::Color => write!(f, "color"),
            Quality::Default => write!(f, "default"),
            Quality::Gray => write!(f, "gray"),
        }
    }
}

enum Rotation {
    Degrees(f32),
}

impl fmt::Display for Rotation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Rotation::Degrees(d) => write!(f, "{d}"),
        }
    }
}
enum Size {
    Width(usize),
}

impl fmt::Display for Size {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Size::Width(w) => write!(f, "{w},"),
        }
    }
}

pub struct Iiif2Url {
    pub identifier: Option<String>,
    region: Region,
    size: Size,
    rotation: Rotation,
    quality: Quality,
    format: Format,
}

impl Iiif2Url {
    pub fn new() -> Self {
        Iiif2Url {
            identifier: None,
            region: Region::Full,
            size: Size::Width(843),
            rotation: Rotation::Degrees(0.0),
            quality: Quality::Default,
            format: Format::JPG,
        }
    }

    pub fn identifier(mut self, identifier: &str) -> Iiif2Url {
        self.identifier = Some(identifier.to_string());
        self
    }

    fn region(mut self, region: Region) -> Iiif2Url {
        self.region = region;
        self
    }

    fn size(mut self, size: Size) -> Iiif2Url {
        self.size = size;
        self
    }

    fn rotation(mut self, rotation: Rotation) -> Iiif2Url {
        self.rotation = rotation;
        self
    }

    fn quality(mut self, quality: Quality) -> Iiif2Url {
        self.quality = quality;
        self
    }

    fn format(mut self, format: Format) -> Iiif2Url {
        self.format = format;
        self
    }
}

impl fmt::Display for Iiif2Url {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "https://www.artic.edu/iiif/2/{}/{}/{}/{}/{}.{}",
            self.identifier.as_ref().unwrap(),
            self.region,
            self.size,
            self.rotation,
            self.quality,
            self.format
        )
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct ImageParams {
    pub image_id: Option<String>,
}

impl ImageParams {
    pub fn new() -> ImageParams {
        ImageParams { image_id: None }
    }
}

pub struct ImageResponse {
    bytes: Bytes,
}

impl ImageResponse {
    pub async fn result(self) -> Result<Bytes> {
        Ok(self.bytes)
    }
}

#[derive(Debug)]
pub struct ImageRequestBuilder {
    client: Client,
    params: ImageParams,
}

impl ImageRequestBuilder {
    pub async fn request(self) -> Result<ImageResponse> {
        Ok(self.client.backend.request(self.params).await)
    }

    pub fn with_image_id(mut self, image_id: String) -> ImageRequestBuilder {
        self.params.image_id = Some(image_id);
        self
    }
}

struct RestBackend {
    client: reqwest::Client,
    headers: HeaderMap,
}

impl RestBackend {
    fn new() -> RestBackend {
        RestBackend {
            client: reqwest::Client::new(),
            headers: {
                let mut headers = HeaderMap::new();
                headers.insert(
                    "user-agent",
                    format!(
                        "AIC-TUI/{} ({}; {})",
                        env!("CARGO_PKG_VERSION"),
                        env!("VERGEN_GIT_DESCRIBE"),
                        env!("VERGEN_BUILD_DATE")
                    )
                    .parse()
                    .unwrap(),
                );
                headers.insert(
                    "AIC-User-Agent",
                    "AIC-TUI (dylan.stark@gmail.com)".parse().unwrap(),
                );
                headers
            },
        }
    }
}

#[async_trait]
impl Backend for RestBackend {
    async fn request(&self, params: ImageParams) -> ImageResponse {
        let url = Iiif2Url::new()
            .identifier(params.image_id.unwrap().as_str())
            .to_string();

        let response = self
            .client
            .get(url)
            .headers(self.headers.clone())
            .send()
            .await
            .expect("request failed");
        let bytes = response.bytes().await.expect("failed to get bytes");
        ImageResponse { bytes }
    }
}

#[async_trait]
trait Backend {
    async fn request(&self, params: ImageParams) -> ImageResponse;
}

impl Debug for dyn Backend + Send {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Backend{{}}")
    }
}

pub struct ClientBuilder {
    backend: Option<Box<dyn Backend + Send>>,
}

impl ClientBuilder {
    pub fn new() -> ClientBuilder {
        ClientBuilder { backend: None }
    }

    pub fn build(self) -> Client {
        let backend = match self.backend {
            Some(backend) => backend,
            None => Box::new(RestBackend::new()),
        };
        Client { backend }
    }

    #[allow(dead_code)]
    fn with_backend(mut self, backend: impl Backend + Send + 'static) -> ClientBuilder {
        self.backend = Some(Box::new(backend));
        self
    }
}

#[derive(Debug)]
pub struct Client {
    backend: Box<dyn Backend + Send>,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub fn image(self) -> ImageRequestBuilder {
        ImageRequestBuilder {
            client: self,
            params: ImageParams::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;

    struct InMemoryBackend {
        request_results: HashMap<ImageParams, Bytes>,
    }

    impl InMemoryBackend {
        fn new() -> InMemoryBackend {
            InMemoryBackend {
                request_results: HashMap::new(),
            }
        }

        fn with_request_result(mut self, params: ImageParams, result: Bytes) -> InMemoryBackend {
            self.request_results.insert(params, result.to_owned());
            self
        }
    }

    #[async_trait]
    impl Backend for InMemoryBackend {
        async fn request(&self, params: ImageParams) -> ImageResponse {
            let bytes = match self.request_results.get(&params) {
                Some(result) => result.clone(),
                None => Bytes::new(),
            };
            ImageResponse { bytes }
        }
    }

    fn in_memory_backend() -> InMemoryBackend {
        let params = ImageParams {
            image_id: Some(String::from("some-image-id")),
        };
        let result = Bytes::from("Some image bytes");
        InMemoryBackend::new().with_request_result(params, result)
    }

    #[test]
    fn test_iiif_url_with_defaults() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/843,/0/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_full_region() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .region(Region::Full)
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/843,/0/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_pixel_region() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .region(Region::Pixels(0, 0, 64, 64))
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/0,0,64,64/843,/0/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_percent_region() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .region(Region::Percent(50.0, 50.5, 25.0, 25.5))
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/pct:50,50.5,25,25.5/843,/0/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_size() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .size(Size::Width(640))
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/640,/0/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_rotation() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .rotation(Rotation::Degrees(42.7))
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/843,/42.7/default.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_quality() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .quality(Quality::Gray)
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/843,/0/gray.jpg"
        );
    }

    #[test]
    fn test_iiif_url_with_format() {
        assert_eq!(
            Iiif2Url::new()
                .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
                .format(Format::PNG)
                .to_string(),
            "https://www.artic.edu/iiif/2/b3974542-b9b4-7568-fc4b-966738f61d78/full/843,/0/default.png"
        );
    }

    #[test]
    fn test_create_client_builder() {
        let _: ClientBuilder = Client::builder();
    }

    #[test]
    fn test_build_client_builder_ok() {
        let _: Client = Client::builder().build();
    }

    #[test]
    fn test_build_client_builder_with_in_memory_backend() {
        let _: Client = Client::builder().with_backend(in_memory_backend()).build();
    }

    #[test]
    fn test_create_search_builder() {
        let client = Client::builder().with_backend(in_memory_backend()).build();

        let _: ImageRequestBuilder = client.image();
    }

    #[test]
    fn test_create_search_builder_with_image_id() {
        let image_request_builder = Client::builder()
            .with_backend(in_memory_backend())
            .build()
            .image();

        let image_request_builder: ImageRequestBuilder =
            image_request_builder.with_image_id(String::from("some-image-id"));

        match image_request_builder.params.image_id {
            Some(actual_image_id) => assert_eq!(actual_image_id, String::from("some-image-id")),
            None => panic!("No image id set"),
        }
    }

    #[tokio::test]
    async fn test_image_request_result() {
        let request_response = Client::builder()
            .with_backend(in_memory_backend())
            .build()
            .image()
            .with_image_id(String::from("some-image-id"))
            .request()
            .await
            .unwrap();

        let bytes: Bytes = request_response.result().await.unwrap();

        assert_eq!(bytes, Bytes::from("Some image bytes"));
    }

    /// Tests simple query for "b3974542-b9b4-7568-fc4b-966738f61d78" against real AIC IIIF.
    #[tokio::test]
    #[ignore]
    async fn test_integration_image_wave() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "user-agent",
            format!(
                "AIC-TUI/{} ({}; {})",
                env!("CARGO_PKG_VERSION"),
                env!("VERGEN_GIT_DESCRIBE"),
                env!("VERGEN_BUILD_DATE")
            )
            .parse()
            .unwrap(),
        );
        headers.insert(
            "AIC-User-Agent",
            "AIC-TUI (dylan.stark@gmail.com)".parse().unwrap(),
        );
        let url = Iiif2Url::new()
            .identifier("b3974542-b9b4-7568-fc4b-966738f61d78")
            .to_string();
        let expected_bytes = reqwest::Client::new()
            .get(url)
            .headers(headers)
            .send()
            .await
            .expect("failed to get response")
            .bytes()
            .await
            .expect("failsed to get bytes from response");

        let actual_bytes = Client::builder()
            .build()
            .image()
            .with_image_id(String::from("b3974542-b9b4-7568-fc4b-966738f61d78"))
            .request()
            .await
            .unwrap()
            .bytes;

        assert_eq!(actual_bytes, expected_bytes)
    }
}
