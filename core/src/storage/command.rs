use crate::app::{AppMeta, Runnable};
use rand::Rng;

#[derive(Clone, Debug, PartialEq)]
pub enum StorageCommand {
    Load { query: String },
}

impl Runnable for StorageCommand {
    fn run(&self, app_meta: &mut AppMeta, _rng: &mut impl Rng) -> String {
        match self {
            Self::Load { query } => {
                let lowercase_query = query.to_lowercase();
                if let Some(result) = app_meta.recent().iter().find(|t| {
                    t.name()
                        .value()
                        .map_or(false, |s| s.to_lowercase() == lowercase_query)
                }) {
                    format!("{}", result.display_details())
                } else {
                    format!("No matches for \"{}\"", query)
                }
            }
        }
    }

    fn summarize(&self) -> &str {
        match self {
            Self::Load { .. } => "load",
        }
    }

    fn parse_input(input: &str, _app_meta: &AppMeta) -> Vec<Self> {
        if input.starts_with(char::is_uppercase) {
            vec![Self::Load {
                query: input.to_string(),
            }]
        } else {
            Vec::new()
        }
    }

    fn autocomplete(input: &str, app_meta: &AppMeta) -> Vec<(String, Self)> {
        if !input
            .chars()
            .next()
            .map(char::is_uppercase)
            .unwrap_or_default()
        {
            Vec::new()
        } else {
            let mut suggestions: Vec<String> = app_meta
                .recent()
                .iter()
                .filter_map(|thing| thing.name().value())
                .filter(|word| word.starts_with(input))
                .cloned()
                .collect();

            suggestions.sort();
            suggestions.truncate(10);

            suggestions
                .iter()
                .flat_map(|s| std::iter::repeat(s).zip(Self::parse_input(s.as_str(), app_meta)))
                .map(|(s, c)| (s.clone(), c))
                .collect()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::world::{Location, Npc};

    #[test]
    fn summarize_test() {
        assert_eq!(
            "load",
            StorageCommand::Load {
                query: String::new(),
            }
            .summarize(),
        );
    }

    #[test]
    fn parse_input_test() {
        let app_meta = AppMeta::default();

        assert_eq!(
            vec![StorageCommand::Load {
                query: "Gandalf the Grey".to_string()
            }],
            StorageCommand::parse_input("Gandalf the Grey", &app_meta),
        );

        assert_eq!(
            Vec::<StorageCommand>::new(),
            StorageCommand::parse_input("potato", &app_meta),
        );
    }

    #[test]
    fn autocomplete_test() {
        let mut app_meta = AppMeta::default();

        app_meta.push_recent(
            Npc {
                name: "Potato Johnson".into(),
                ..Default::default()
            }
            .into(),
        );

        app_meta.push_recent(
            Npc {
                name: "potato should be capitalized".into(),
                ..Default::default()
            }
            .into(),
        );

        app_meta.push_recent(
            Location {
                name: "Potato & Potato, Esq.".into(),
                ..Default::default()
            }
            .into(),
        );

        app_meta.push_recent(
            Location {
                name: "Spud Stop".into(),
                ..Default::default()
            }
            .into(),
        );

        assert_eq!(
            vec![
                (
                    "Potato & Potato, Esq.".to_string(),
                    StorageCommand::Load {
                        query: "Potato & Potato, Esq.".to_string(),
                    }
                ),
                (
                    "Potato Johnson".to_string(),
                    StorageCommand::Load {
                        query: "Potato Johnson".to_string(),
                    }
                ),
            ],
            StorageCommand::autocomplete("P", &app_meta),
        );

        assert!(StorageCommand::autocomplete("p", &app_meta).is_empty());
        assert!(StorageCommand::autocomplete("", &app_meta).is_empty());
    }
}
