#[cfg(test)]
pub mod tests {
    use crate::artworks::Collection;

    pub fn basic_pagination() -> serde_json::Value {
        serde_json::json!(
            {
                "total": 1,
                "limit": 12,
                "offset": 0,
                "total_pages": 42,
                "current_page": 1,
                "next_url": "https://www.artic.edu/artworks/?page=2"
            }
        )
    }

    pub fn numero_uno() -> serde_json::Value {
        serde_json::json!(
            {
                "id": 1,
                "title": "Numero uno"
            }
        )
    }

    pub fn numero_tres() -> serde_json::Value {
        serde_json::json!(
            {
                "id": 3,
                "title": "Numero tres"
            }
        )
    }

    pub fn collection_with_numero_uno() -> Collection {
        Collection::new(serde_json::json!({
            "pagination": basic_pagination(),
            "data": vec![numero_uno()],
        }))
    }

    pub fn collection_with_numeros_uno_and_tres() -> Collection {
        Collection::new(serde_json::json!({
            "pagination": basic_pagination(),
            "data": vec![numero_uno(), numero_tres()],
        }))
    }
}
