# iiif

Build requests for servers that implement the [IIIF Image API 2.0](https://iiif.io/api/image/2.0/): an image's information (`info.json`) and the image itself, cropped, scaled, rotated and in the quality and format you choose.

You bring the base URI, and the builder makes a request that is valid by construction: a region, size or rotation that doesn't parse is an error, not a broken URL.

```rust
use iiif::{BaseUri, Format, ImageRequest, InformationRequest, Quality, Region, Size};

let uri: BaseUri = "https://www.artic.edu/iiif/2/3eec80ab-bad4-8b35-1adb-57b7e7cb7adb".parse()?;
println!("{}", InformationRequest::new(uri.clone()));

let image = ImageRequest::builder()
    .base_uri(uri)
    .region(Region::Full)
    .size(Size::Width(843))
    .quality(Quality::Default)
    .format(Format::Jpg)
    .build();
println!("{image}"); // https://www.artic.edu/iiif/2/3eec…/full/843,/0/default.jpg
```

Full URLs parse back into requests too, and the `serde` feature serializes every type as its IIIF string form.

`iiif` has no dependency on the rest of the [acres](https://github.com/dylan-stark/acres) workspace, so it works with any IIIF 2.0 server. `cargo doc --open -p iiif` builds the API docs, which cover every type.

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT license](../LICENSE-MIT) at your option.
