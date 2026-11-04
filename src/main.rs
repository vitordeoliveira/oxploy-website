mod app;
// Installed components come with variants and helpers the site may not use yet.
#[allow(dead_code)]
mod components;
mod features;

#[tokio::main]
async fn main() {
    topcoat::start(app::router().build()).await.unwrap();
}
