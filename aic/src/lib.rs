mod artworks;
mod config;

use crate::artworks::ArtworksListing;
use crate::config::Config;

pub struct Api {
    base_uri: String,
    use_cache: bool,
}

impl Api {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> ApiBuilder {
        ApiBuilder::default()
    }

    pub fn use_cache(&self) -> bool {
        self.use_cache
    }

    pub async fn artworks(&self) -> ArtworksListing {
        let config = Config::new().unwrap();
        let artworks_json_path = config.aic_cache_dir.join("artworks.json");
        println!("artworks_json_path: {:?}", artworks_json_path);
        if self.use_cache && artworks_json_path.is_file() {
            let json = std::fs::read_to_string(artworks_json_path).unwrap();
            serde_json::from_str(&json).unwrap()
        } else {
            let artworks_path = format!("{}/artworks", self.base_uri);
            eprintln!("artworks_path: {}", artworks_path);
            let client = reqwest::Client::new();
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "user-agent",
                format!("ACRES/{}", env!("CARGO_PKG_VERSION"),)
                    .parse()
                    .unwrap(),
            );
            headers.insert(
                "ACRES-User-Agent",
                "ACRES (dylan.stark@gmail.com)".parse().unwrap(),
            );

            let listing = client
                .get(artworks_path)
                .headers(headers)
                .send()
                .await
                .unwrap()
                .json::<ArtworksListing>()
                .await
                .unwrap();
            std::fs::create_dir_all(artworks_json_path.parent().unwrap()).unwrap();
            std::fs::write(artworks_json_path, listing.to_string()).unwrap();
            listing
        }
    }
}

impl Default for Api {
    fn default() -> Self {
        ApiBuilder::default().build()
    }
}

pub struct ApiBuilder {
    base_uri: String,
    use_cache: bool,
}

impl ApiBuilder {
    pub fn base_uri(mut self, base_uri: &str) -> Self {
        self.base_uri = base_uri.to_string();
        self
    }

    pub fn use_cache(mut self, use_cache: bool) -> Self {
        self.use_cache = use_cache;
        self
    }

    pub fn build(self) -> Api {
        Api {
            base_uri: self.base_uri,
            use_cache: self.use_cache,
        }
    }
}

impl Default for ApiBuilder {
    fn default() -> Self {
        ApiBuilder {
            base_uri: String::from("https://api.artic.edu/api/v1"),
            use_cache: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use wiremock::matchers::{any, path};
    use wiremock::{Mock, ResponseTemplate};

    use super::*;

    #[test]
    fn use_cache_by_default() {
        assert!(Api::new().use_cache());
    }

    #[test]
    fn custom_base_uri() {
        let custom_uri = "http://localhost:80/api/v1";
        let api = Api::builder().base_uri(custom_uri).build();
        assert_eq!(api.base_uri, custom_uri);
    }

    #[tokio::test]
    async fn api_artworks_listing() {
        let mock_server = wiremock::MockServer::start().await;
        let mock_uri = format!("{}/api/v1", mock_server.uri());
        let mock_listing: ArtworksListing =
            serde_json::from_str(r#"{ "data": [ { "id": 1, "title": "Numero uno" } ] }"#).unwrap();
        Mock::given(any())
            .and(path("/api/v1/artworks"))
            .respond_with(ResponseTemplate::new(200).set_body_json(mock_listing))
            .expect(1)
            .mount(&mock_server)
            .await;
        let api = Api::builder().base_uri(&mock_uri).use_cache(false).build();
        assert_eq!(api.base_uri, mock_uri);

        let listing: ArtworksListing = api.artworks().await;

        assert_eq!(listing.data.len(), 1);
        assert_eq!(listing.data[0].id, 1);
        assert_eq!(listing.data[0].title, "Numero uno");
    }
}
