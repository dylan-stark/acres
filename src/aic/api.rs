pub mod artworks {
    use std::{collections::HashMap, fmt::Debug};

    use reqwest::header::HeaderMap;
    use serde::{Deserialize, Serialize};

    use crate::aic::Result;

    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    pub struct Artwork {
        pub id: usize,
        pub image_id: String,
        pub title: String,
    }

    #[derive(Debug, Clone, Eq, Hash, PartialEq)]
    pub struct SearchParams {
        pub text: Option<String>,
    }

    impl SearchParams {
        pub fn new() -> SearchParams {
            SearchParams { text: None }
        }
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Datum {
        _score: f32,
        id: usize,
        image_id: String,
        title: String,
    }

    impl PartialEq for Datum {
        fn eq(&self, other: &Self) -> bool {
            self.id == other.id && self.image_id == other.image_id && self.title == other.title
        }
    }

    impl Eq for Datum {}

    #[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
    pub struct SearchResults {
        data: Vec<Datum>,
    }

    #[derive(Debug)]
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
        async fn search(&self, params: SearchParams) -> SearchResponse {
            let mut url = "https://api.artic.edu/api/v1/artworks/search".to_string();
            url = format!("{}?q={}", url, params.text.unwrap());
            url = format!(
                "{}&query[term][is_public_domain]=true&fields=id,title,image_id",
                url
            );

            let response = self
                .client
                .get(url)
                .headers(self.headers.clone())
                .send()
                .await
                .expect("search failed");
            let body = response.text().await.expect("failed to get text");
            SearchResponse { body }
        }
    }

    #[derive(Debug)]
    pub struct SearchRequestBuilder {
        client: Client,
        params: SearchParams,
    }

    impl SearchRequestBuilder {
        /// Starts the search to the get the response.
        ///
        /// If we have a response already for these search params, then we just
        /// return that. Otherwise, we hit the REST API.
        pub async fn start(self) -> Result<SearchResponse> {
            match self.client.search_store.get(&self.params) {
                Some(body) => Ok(SearchResponse {
                    body: body.to_string(),
                }),
                None => Ok(self.client.backend.search(self.params).await),
            }
        }

        pub fn with_text(mut self, text: String) -> SearchRequestBuilder {
            self.params.text = Some(text);
            self
        }
    }

    pub struct SearchResponse {
        body: String,
    }

    impl SearchResponse {
        pub async fn result(self) -> Result<Vec<Artwork>> {
            let result: SearchResults =
                serde_json::from_str(self.body.as_str()).expect("failed to parse response body");
            Ok(result
                .data
                .iter()
                .map(|datum| Artwork {
                    id: datum.id,
                    title: datum.title.clone(),
                    image_id: datum.image_id.clone(),
                })
                .collect::<Vec<Artwork>>())
        }
    }

    #[derive(Debug)]
    pub struct Client {
        search_store: HashMap<SearchParams, String>,
        backend: RestBackend,
    }

    impl Client {
        pub fn builder() -> ClientBuilder {
            ClientBuilder::new()
        }

        pub fn search(self) -> SearchRequestBuilder {
            SearchRequestBuilder {
                client: self,
                params: SearchParams::new(),
            }
        }
    }

    pub struct ClientBuilder {
        backend: RestBackend,
    }

    impl ClientBuilder {
        pub fn new() -> ClientBuilder {
            ClientBuilder {
                backend: RestBackend::new(),
            }
        }

        pub fn build(self) -> Client {
            let search_store = HashMap::new();
            Client {
                search_store,
                backend: self.backend,
            }
        }
    }

    #[cfg(test)]
    mod test {
        use super::*;

        fn load_simple_example_to_store(client: &mut Client) {
            let params = SearchParams {
                text: Some(String::from("some query")),
            };
            let results: &'static str = r###"
            {
                "data": [
                    {
                        "_score": 122.88055,
                        "id": 24645,
                        "image_id": "b3974542-b9b4-7568-fc4b-966738f61d78",
                        "title": "Under the Wave off Kanagawa ...(Fugaku sanj\u016brokkei)\""
                    }
                ]
            }
            "###;
            client.search_store.insert(params, results.to_string());
        }

        #[test]
        fn test_artworks_to_json() {
            let artworks_list = vec![
                Artwork {
                    id: 123_usize,
                    image_id: String::from("123-image-id"),
                    title: String::from("123-title"),
                },
                Artwork {
                    id: 456_usize,
                    image_id: String::from("456-image-id"),
                    title: String::from("456-title"),
                },
            ];

            let actual_json: String = serde_json::to_string(&artworks_list).unwrap();

            let expected_json = String::from(
                "[{\"id\":123,\"image_id\":\"123-image-id\",\"title\":\"123-title\"}\
                ,{\"id\":456,\"image_id\":\"456-image-id\",\"title\":\"456-title\"}]",
            );
            assert_eq!(actual_json, expected_json)
        }

        #[test]
        fn test_artworks_from_json() {
            let json = "[{\"id\":123,\"image_id\":\"123-image-id\",\"title\":\"123-title\"}\
                ,{\"id\":456,\"image_id\":\"456-image-id\",\"title\":\"456-title\"}]";

            let actual_artworks_list: Vec<Artwork> = serde_json::from_str(json).unwrap();

            let expected_artworks_list = vec![
                Artwork {
                    id: 123_usize,
                    image_id: String::from("123-image-id"),
                    title: String::from("123-title"),
                },
                Artwork {
                    id: 456_usize,
                    image_id: String::from("456-image-id"),
                    title: String::from("456-title"),
                },
            ];
            assert_eq!(actual_artworks_list, expected_artworks_list)
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
        fn test_create_search_builder() {
            let client = Client::builder().build();

            let _: SearchRequestBuilder = client.search();
        }

        #[test]
        fn test_create_search_builder_with_text() {
            let search_builder = Client::builder().build().search();

            let search_builder: SearchRequestBuilder =
                search_builder.with_text(String::from("some query"));

            match search_builder.params.text {
                Some(actual_text) => assert_eq!(actual_text, String::from("some query")),
                None => panic!("No text set"),
            }
        }

        // TODO: Adjust this to test the real behavior we want, not
        //       the this implementation detail.
        #[tokio::test]
        #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
        async fn test_search_response_result_panics_without_text() {
            let mut client = Client::builder().build();
            load_simple_example_to_store(&mut client);
            let search_response = client.search().start().await.unwrap();

            let _ = search_response.result().await;
        }

        #[tokio::test]
        async fn test_search_response_result() {
            let mut client = Client::builder().build();
            load_simple_example_to_store(&mut client);
            let search_response = client
                .search()
                .with_text(String::from("some query"))
                .start()
                .await
                .unwrap();

            let pieces: Vec<Artwork> = search_response.result().await.unwrap();

            assert!(pieces.len() == 1);
            let Artwork {
                id,
                image_id,
                title,
            } = &pieces[0];
            assert_eq!(*id, 24645_usize);
            assert_eq!(image_id, "b3974542-b9b4-7568-fc4b-966738f61d78");
            assert_eq!(
                title,
                "Under the Wave off Kanagawa ...(Fugaku sanj\u{016b}rokkei)\""
            );
        }

        /// Tests simple query for "wave" against real AIC API.
        ///
        /// We only check that the body we expect to get is the same
        /// as that provided by our REST backend. The logic for processing
        /// the body is already tested.
        #[tokio::test]
        #[ignore]
        async fn test_integration_search_wave() {
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
            let expected_body = reqwest::Client::new()
                .get("https://api.artic.edu/api/v1/artworks/search?q=wave&query[term][is_public_domain]=true&fields=id,title,image_id")
                .headers(headers)
                .send()
                .await
                .expect("failed to get response")
                .text()
                .await
                .expect("failsed to get text from response");

            let actual_body = Client::builder()
                .build()
                .search()
                .with_text(String::from("wave"))
                .start()
                .await
                .unwrap()
                .body;

            assert_eq!(actual_body, expected_body)
        }
    }
}
