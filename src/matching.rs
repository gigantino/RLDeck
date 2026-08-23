use crate::{catalog, library};

pub fn catalog_star_key(name: &str) -> String {
    format!("catalog:{}", name.trim().to_lowercase())
}

fn catalog_identity(name: &str) -> String {
    name.chars().filter(|character| character.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn catalog_words(name: &str) -> Vec<String> {
    name.split(|character: char| !character.is_alphanumeric()).filter(|word| !word.is_empty()).map(str::to_lowercase).collect()
}

fn shortened_catalog_title_matches(local: &str, catalog: &str) -> bool {
    let local = catalog_words(local);
    let catalog = catalog_words(catalog);
    let shorter = local.len().min(catalog.len());

    shorter >= 2 && local[..shorter] == catalog[..shorter] && local.len().abs_diff(catalog.len()) <= 2
}

fn matching_catalog_entry<'a>(name: &str, catalog: &'a [catalog::Entry]) -> Option<&'a catalog::Entry> {
    let identity = catalog_identity(name);

    if let Some(exact) = catalog.iter().find(|entry| catalog_identity(&entry.name) == identity) {
        return Some(exact);
    }

    let mut candidates = catalog.iter().filter(|entry| shortened_catalog_title_matches(name, &entry.name));
    let only = candidates.next()?;
    candidates.next().is_none().then_some(only)
}

pub fn reconcile_catalog_metadata(library: &mut [library::Map], catalog: &[catalog::Entry]) {
    for map in library {
        let Some(entry) = matching_catalog_entry(&map.name, catalog) else {
            continue;
        };

        map.author.get_or_insert_with(|| "Lethamyr".to_string());
        if map.blurb.is_none() {
            map.blurb = entry.description_short.as_deref().map(catalog::plain_text);
        }
        if map.description.is_none() {
            map.description = entry.description.as_deref().map(catalog::plain_text);
        }
    }
}
