use crate::app::{Autocomplete, Context};
use std::str::FromStr;

#[derive(Debug)]
pub enum Command {
    Load { query: String },
}

impl Command {
    pub fn run(&self, context: &mut Context) -> String {
        match self {
            Self::Load { query } => {
                let lowercase_query = query.to_lowercase();
                if let Some(result) = context.recent().iter().find(|t| {
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
}

impl FromStr for Command {
    type Err = ();

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if raw.starts_with(char::is_uppercase) {
            Ok(Self::Load {
                query: raw.to_string(),
            })
        } else {
            Err(())
        }
    }
}

impl Autocomplete for Command {
    fn autocomplete(input: &str, context: &Context) -> Vec<String> {
        if !input
            .chars()
            .next()
            .map(char::is_uppercase)
            .unwrap_or_default()
        {
            Vec::new()
        } else {
            let mut suggestions: Vec<String> = context
                .recent()
                .iter()
                .filter_map(|thing| thing.name().value())
                .filter(|word| word.starts_with(input))
                .cloned()
                .collect();

            suggestions.sort();
            suggestions.truncate(10);

            suggestions
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::world::{Location, Npc};

    #[test]
    fn from_str_test() {
        let parsed_command = "Gandalf the Grey".parse();
        if let Ok(Command::Load { query }) = parsed_command {
            assert_eq!("Gandalf the Grey", query.as_str());
        } else {
            panic!("{:?}", parsed_command);
        }

        let parsed_command = "potato".parse::<Command>();
        assert!(matches!(parsed_command, Err(())), "{:?}", parsed_command);
    }

    #[test]
    fn autocomplete_test() {
        let mut context = Context::default();

        context.push_recent(
            Npc {
                name: "Potato Johnson".into(),
                ..Default::default()
            }
            .into(),
        );

        context.push_recent(
            Npc {
                name: "potato should be capitalized".into(),
                ..Default::default()
            }
            .into(),
        );

        context.push_recent(
            Location {
                name: "Potato & Potato, Esq.".into(),
                ..Default::default()
            }
            .into(),
        );

        context.push_recent(
            Location {
                name: "Spud Stop".into(),
                ..Default::default()
            }
            .into(),
        );

        assert_eq!(
            vec!["Potato & Potato, Esq.", "Potato Johnson"],
            Command::autocomplete("P", &context),
        );

        assert_eq!(Vec::<String>::new(), Command::autocomplete("p", &context));
        assert_eq!(Vec::<String>::new(), Command::autocomplete("", &context));
    }
}
