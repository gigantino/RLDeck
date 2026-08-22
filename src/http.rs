use std::sync::OnceLock;

pub const USER_AGENT: &str = concat!("RLDeck/", env!("CARGO_PKG_VERSION"), " (+rocket league map loader)");

pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

    CLIENT.get_or_init(|| reqwest::Client::builder().user_agent(USER_AGENT).build().unwrap_or_default())
}
