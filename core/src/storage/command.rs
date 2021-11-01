use crate::app::{AppMeta, Autocomplete, CommandAlias, ContextAwareParse, Runnable};
use crate::storage::{Change, RepositoryError};
use crate::world::Thing;
use async_trait::async_trait;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum StorageCommand {
    Change { change: Change },
    Journal,
    Load { name: String },
    Redo,
    Undo,
}

impl StorageCommand {
    fn summarize(
        &self,
        thing: Option<&Thing>,
        undo_change: Option<&Change>,
        redo_change: Option<&Change>,
    ) -> String {
        match (self, thing) {
            (
                Self::Change {
                    change: Change::Delete { .. },
                },
                Some(thing),
            ) => format!("remove {} from journal", thing.as_str()),
            (
                Self::Change {
                    change: Change::Delete { .. },
                },
                None,
            ) => "remove an entry from journal".to_string(),
            (
                Self::Change {
                    change: Change::Save { .. },
                },
                Some(thing),
            ) => format!("save {} to journal", thing.as_str()),
            (
                Self::Change {
                    change: Change::Save { .. },
                },
                None,
            ) => "save an entry to journal".to_string(),
            (Self::Change { .. }, _) => unreachable!(),
            (Self::Journal { .. }, _) => "list journal contents".to_string(),
            (Self::Load { .. }, Some(thing)) => {
                if thing.uuid().is_some() {
                    format!("{}", thing.display_description())
                } else {
                    format!("{} (unsaved)", thing.display_description())
                }
            }
            (Self::Load { .. }, None) => "load an entry".to_string(),
            (Self::Redo, _) => {
                if let Some(change) = redo_change {
                    format!("redo {}", change.display_redo())
                } else {
                    "nothing to redo".to_string()
                }
            }
            (Self::Undo, _) => {
                if let Some(change) = undo_change {
                    format!("undo {}", change.display_undo())
                } else {
                    "nothing to undo".to_string()
                }
            }
        }
    }
}

#[async_trait(?Send)]
impl Runnable for StorageCommand {
    async fn run(self, _input: &str, app_meta: &mut AppMeta) -> Result<String, String> {
        match self {
            Self::Journal => {
                if !app_meta.repository.data_store_enabled() {
                    return Err("The journal is not supported by your browser.".to_string());
                }

                let mut output = "# Journal".to_string();
                let [mut npcs, mut places, mut regions] = [Vec::new(), Vec::new(), Vec::new()];

                let record_count = app_meta
                    .repository
                    .journal()
                    .map(|thing| match thing {
                        Thing::Npc(_) => npcs.push(thing),
                        Thing::Place(_) => places.push(thing),
                        Thing::Region(_) => regions.push(thing),
                    })
                    .count();

                let mut add_section = |title: &str, mut things: Vec<&Thing>| {
                    if !things.is_empty() {
                        output.push_str("\n\n## ");
                        output.push_str(title);

                        things.sort_unstable_by_key(|t| t.name().value());

                        things.drain(..).enumerate().for_each(|(i, thing)| {
                            if i > 0 {
                                output.push('\\');
                            }

                            output.push_str(&format!("\n{}", thing.display_summary()));
                        });
                    }
                };

                add_section("NPCs", npcs);
                add_section("Places", places);
                add_section("Regions", regions);

                if record_count == 0 {
                    output.push_str("\n\n*Your journal is currently empty.*");
                }

                Ok(output)
            }
            Self::Change { change } => {
                if matches!(change, Change::Save { .. } | Change::Unsave { .. })
                    && !app_meta.repository.data_store_enabled()
                {
                    return Err("The journal is not supported by your browser.".to_string());
                }

                let name = match &change {
                    Change::Create { thing } | Change::CreateAndSave { thing } => {
                        thing.name().to_string()
                    }
                    Change::Delete { name, .. }
                    | Change::Edit { name, .. }
                    | Change::EditAndUnsave { name, .. }
                    | Change::Save { name }
                    | Change::Unsave { name, .. } => name.to_owned(),
                };

                match &change {
                    Change::Create { .. } | Change::CreateAndSave { .. } => app_meta
                        .repository
                        .modify(change)
                        .await
                        .map(|_| format!("{} was successfully restored. Use `undo` to reverse this.", name))
                        .map_err(|_| format!("Couldn't restore {}.", name)),
                    Change::Delete { .. } => app_meta
                        .repository
                        .modify(change)
                        .await
                        .map(|_| format!("{} was successfully deleted. Use `undo` to reverse this.", name))
                        .map_err(|(_, e)| match e {
                            RepositoryError::NotFound => {
                                format!("There is no entity named \"{}\".", name)
                            }
                            RepositoryError::DataStoreFailed
                            | RepositoryError::MissingName
                            | RepositoryError::NameAlreadyExists => {
                                format!("Couldn't delete `{}`.", name)
                            }
                        }),
                    Change::Edit { .. } | Change::EditAndUnsave { .. } => {
                        let thing_type = if let Change::Edit { ref diff, .. } | Change::EditAndUnsave { ref diff, .. } = change {
                            diff.as_str()
                        } else {
                            unreachable!()
                        };

                        app_meta
                        .repository
                        .modify(change)
                        .await
                        .map(|id| {
                            let thing = app_meta.repository.load(&id).unwrap();

                            if matches!(app_meta.repository.undo_history().next(), Some(Change::EditAndUnsave { .. })) {
                                format!(
                                    "{}\n\n_{} was successfully edited and automatically saved to your `journal`. Use `undo` to reverse this._",
                                    thing.display_details(),
                                    name,
                                )
                            } else {
                                format!(
                                    "{}\n\n_{} was successfully edited. Use `undo` to reverse this._",
                                    thing.display_details(),
                                    name,
                                )
                            }
                        })
                        .map_err(|(_, e)| match e {
                            RepositoryError::NotFound => {
                                format!("There is no {} named \"{}\".", thing_type, name)
                            }
                            RepositoryError::DataStoreFailed
                                | RepositoryError::MissingName
                                | RepositoryError::NameAlreadyExists => {
                                format!("Couldn't edit `{}`.", name)
                                }
                        })
                    }
                    Change::Save { .. } => app_meta
                        .repository
                        .modify(change)
                        .await
                        .map(|_| format!("{} was successfully saved. Use `undo` to reverse this.", name))
                        .map_err(|(_, e)| match e {
                            RepositoryError::NotFound => {
                                format!("There is no entity named \"{}\".", name)
                            }
                            RepositoryError::DataStoreFailed
                            | RepositoryError::MissingName
                            | RepositoryError::NameAlreadyExists => {
                                format!("Couldn't save `{}`.", name)
                            }
                        }),
                    Change::Unsave { .. } => app_meta
                        .repository
                        .modify(change)
                        .await
                        .map(|_| format!("{} was successfully removed from the journal. Use `undo` to reverse this.", name))
                        .map_err(|_| format!("Couldn't remove {} from the journal.", name)),
                }
            }
            Self::Load { name } => {
                let thing = app_meta.repository.load(&name.as_str().into());
                let mut save_command = None;
                let output = if let Some(thing) = thing {
                    if thing.uuid().is_none() && app_meta.repository.data_store_enabled() {
                        save_command = Some(CommandAlias::literal(
                            "save".to_string(),
                            format!("save {}", name),
                            StorageCommand::Change {
                                change: Change::Save { name },
                            }
                            .into(),
                        ));

                        Ok(format!(
                            "{}\n\n_{} has not yet been saved. Use ~save~ to save {} to your `journal`._",
                            thing.display_details(),
                            thing.name(),
                            thing.gender().them(),
                        ))
                    } else {
                        Ok(format!("{}", thing.display_details()))
                    }
                } else {
                    Err(format!("No matches for \"{}\"", name))
                };

                if let Some(save_command) = save_command {
                    app_meta.command_aliases.insert(save_command);
                }

                output
            }
            Self::Redo => match app_meta.repository.redo().await {
                Some(Ok(id)) => {
                    let action = app_meta
                        .repository
                        .undo_history()
                        .next()
                        .unwrap()
                        .display_undo();

                    if let Some(thing) = app_meta.repository.load(&id) {
                        Ok(format!(
                            "{}\n\n_Successfully redid {}. Use `undo` to reverse this._",
                            thing.display_details(),
                            action,
                        ))
                    } else {
                        Ok(format!(
                            "Successfully redid {}. Use `undo` to reverse this.",
                            action,
                        ))
                    }
                }
                Some(Err(_)) => Err("Failed to redo.".to_string()),
                None => Err("Nothing to redo.".to_string()),
            },
            Self::Undo => match app_meta.repository.undo().await {
                Some(Ok(id)) => {
                    let action = app_meta.repository.get_redo().unwrap().display_redo();

                    if let Some(thing) = app_meta.repository.load(&id) {
                        Ok(format!(
                            "{}\n\n_Successfully undid {}. Use `redo` to reverse this._",
                            thing.display_details(),
                            action,
                        ))
                    } else {
                        Ok(format!(
                            "Successfully undid {}. Use `redo` to reverse this.",
                            action,
                        ))
                    }
                }
                Some(Err(_)) => Err("Failed to undo.".to_string()),
                None => Err("Nothing to undo.".to_string()),
            },
        }
    }
}

impl ContextAwareParse for StorageCommand {
    fn parse_input(input: &str, app_meta: &AppMeta) -> (Option<Self>, Vec<Self>) {
        let mut fuzzy_matches = Vec::new();

        if input.starts_with(char::is_uppercase)
            && app_meta.repository.load(&input.into()).is_some()
        {
            fuzzy_matches.push(Self::Load {
                name: input.to_string(),
            });
        }

        (
            if let Some(name) = input.strip_prefix("delete ") {
                Some(Self::Change {
                    change: Change::Delete {
                        id: name.into(),
                        name: name.to_string(),
                    },
                })
            } else if let Some(name) = input.strip_prefix("load ") {
                Some(Self::Load {
                    name: name.to_string(),
                })
            } else if let Some(name) = input.strip_prefix("save ") {
                Some(Self::Change {
                    change: Change::Save {
                        name: name.to_string(),
                    },
                })
            } else {
                match input {
                    "journal" => Some(Self::Journal),
                    "undo" => Some(Self::Undo),
                    "redo" => Some(Self::Redo),
                    _ => None,
                }
            },
            fuzzy_matches,
        )
    }
}

impl Autocomplete for StorageCommand {
    fn autocomplete(input: &str, app_meta: &AppMeta) -> Vec<(String, String)> {
        let mut suggestions = Vec::new();

        ["delete", "load", "save"]
            .iter()
            .filter(|s| s.starts_with(input))
            .filter_map(|s| {
                let suggestion = format!("{} [name]", s);
                Self::parse_input(&suggestion, app_meta)
                    .0
                    .map(|command| (suggestion, command))
            })
            .chain(
                ["journal", "undo", "redo"]
                    .iter()
                    .filter(|s| s.starts_with(input))
                    .filter_map(|s| Self::parse_input(s, app_meta).0.map(|c| (s.to_string(), c))),
            )
            .for_each(|(s, command)| {
                suggestions.push((
                    s,
                    command.summarize(
                        None,
                        app_meta.repository.undo_history().next(),
                        app_meta.repository.get_redo(),
                    ),
                ))
            });

        let (input_prefix, input_name) = if let Some(parts) = ["delete ", "load ", "save "]
            .iter()
            .find_map(|prefix| input.strip_prefix(prefix).map(|name| (*prefix, name)))
        {
            parts
        } else {
            ("", input)
        };

        app_meta
            .repository
            .all()
            .filter_map(|thing| {
                thing
                    .name()
                    .value()
                    .map(|name| {
                        if name.starts_with(input_name) {
                            match (input_prefix, thing.uuid()) {
                                ("save ", Some(_)) | ("delete ", None) => None,
                                _ => Some((input_prefix, thing)),
                            }
                        } else if name.starts_with(input) {
                            Some(("", thing))
                        } else {
                            None
                        }
                    })
                    .flatten()
            })
            .filter_map(|(prefix, thing)| {
                let suggestion = format!("{}{}", prefix, thing.name());
                let (exact_match, mut fuzzy_matches) = Self::parse_input(&suggestion, app_meta);

                exact_match
                    .or_else(|| fuzzy_matches.drain(..).next())
                    .map(|command| (suggestion, thing, command))
            })
            .take(10)
            .for_each(|(suggestion, thing, command)| {
                suggestions.push((suggestion, command.summarize(Some(thing), None, None)))
            });

        suggestions
    }
}

impl fmt::Display for StorageCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            Self::Change {
                change: Change::Delete { name, .. },
            } => write!(f, "delete {}", name),
            Self::Change {
                change: Change::Save { name },
            } => write!(f, "save {}", name),
            Self::Change { .. } => unreachable!(),
            Self::Journal => write!(f, "journal"),
            Self::Load { name } => write!(f, "load {}", name),
            Self::Redo => write!(f, "redo"),
            Self::Undo => write!(f, "undo"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::storage::NullDataStore;
    use crate::world::npc::{Age, Gender, Npc, Species};
    use crate::world::place::{Place, PlaceType};
    use crate::world::Thing;
    use tokio_test::block_on;
    use uuid::Uuid;

    #[test]
    fn summarize_test() {
        {
            let mut place = Place {
                subtype: PlaceType::Inn.into(),
                ..Default::default()
            }
            .into();

            assert_eq!(
                "inn (unsaved)",
                StorageCommand::Load {
                    name: String::new(),
                }
                .summarize(Some(&place), None, None),
            );

            place.set_uuid(Uuid::new_v4());

            assert_eq!(
                "inn",
                StorageCommand::Load {
                    name: String::new(),
                }
                .summarize(Some(&place), None, None),
            );

            assert_eq!(
                "save place to journal",
                StorageCommand::Change {
                    change: Change::Save {
                        name: String::new(),
                    },
                }
                .summarize(Some(&place), None, None),
            );
        }

        {
            let mut npc = Npc {
                species: Species::Gnome.into(),
                ..Default::default()
            }
            .into();

            assert_eq!(
                "gnome (unsaved)",
                StorageCommand::Load {
                    name: String::new(),
                }
                .summarize(Some(&npc), None, None),
            );

            npc.set_uuid(Uuid::new_v4());

            assert_eq!(
                "gnome",
                StorageCommand::Load {
                    name: String::new(),
                }
                .summarize(Some(&npc), None, None),
            );

            assert_eq!(
                "save character to journal",
                StorageCommand::Change {
                    change: Change::Save {
                        name: String::new(),
                    },
                }
                .summarize(Some(&npc), None, None),
            );
        }

        {
            let mut region = Thing::Region(Default::default());

            assert_eq!(
                "region (unsaved)",
                StorageCommand::Load {
                    name: String::new(),
                }
                .summarize(Some(&region), None, None),
            );

            region.set_uuid(Uuid::new_v4());

            assert_eq!(
                "region",
                StorageCommand::Load {
                    name: String::new(),
                }
                .summarize(Some(&region), None, None),
            );

            assert_eq!(
                "save region to journal",
                StorageCommand::Change {
                    change: Change::Save {
                        name: String::new(),
                    }
                }
                .summarize(Some(&region), None, None),
            );
        }

        {
            assert_eq!(
                "load an entry",
                StorageCommand::Load {
                    name: String::new(),
                }
                .summarize(None, None, None),
            );

            assert_eq!(
                "save an entry to journal",
                StorageCommand::Change {
                    change: Change::Save {
                        name: String::new(),
                    }
                }
                .summarize(None, None, None),
            );
        }

        {
            let change = Change::Save {
                name: "Potato Johnson".to_string(),
            };

            assert_eq!(
                "undo removing Potato Johnson from journal",
                StorageCommand::Undo.summarize(None, Some(&change), None),
            );

            assert_eq!(
                "nothing to undo",
                StorageCommand::Undo.summarize(None, None, None),
            );
        }

        {
            let change = Change::Save {
                name: "Potato Johnson".to_string(),
            };

            assert_eq!(
                "redo saving Potato Johnson to journal",
                StorageCommand::Redo.summarize(None, None, Some(&change)),
            );

            assert_eq!(
                "nothing to redo",
                StorageCommand::Redo.summarize(None, None, None),
            );
        }
    }

    #[test]
    fn parse_input_test() {
        let app_meta = AppMeta::new(NullDataStore::default());

        assert_eq!(
            (Option::<StorageCommand>::None, Vec::new()),
            StorageCommand::parse_input("Gandalf the Grey", &app_meta),
        );

        assert_eq!(
            (
                Some(StorageCommand::Change {
                    change: Change::Delete {
                        id: "Gandalf the Grey".into(),
                        name: "Gandalf the Grey".to_string(),
                    },
                }),
                Vec::new(),
            ),
            StorageCommand::parse_input("delete Gandalf the Grey", &app_meta),
        );

        assert_eq!(
            (
                Some(StorageCommand::Change {
                    change: Change::Save {
                        name: "Gandalf the Grey".to_string(),
                    },
                }),
                Vec::new(),
            ),
            StorageCommand::parse_input("save Gandalf the Grey", &app_meta),
        );

        assert_eq!(
            (
                Some(StorageCommand::Load {
                    name: "Gandalf the Grey".to_string()
                }),
                Vec::new(),
            ),
            StorageCommand::parse_input("load Gandalf the Grey", &app_meta),
        );

        assert_eq!(
            (Some(StorageCommand::Journal), Vec::new()),
            StorageCommand::parse_input("journal", &app_meta),
        );

        assert_eq!(
            (None, Vec::<StorageCommand>::new()),
            StorageCommand::parse_input("potato", &app_meta),
        );
    }

    #[test]
    fn autocomplete_test() {
        let mut app_meta = AppMeta::new(NullDataStore::default());

        block_on(
            app_meta.repository.modify(Change::Create {
                thing: Npc {
                    name: "Potato Johnson".into(),
                    species: Species::Elf.into(),
                    gender: Gender::NonBinaryThey.into(),
                    age: Age::Adult.into(),
                    ..Default::default()
                }
                .into(),
            }),
        )
        .unwrap();

        block_on(
            app_meta.repository.modify(Change::Create {
                thing: Npc {
                    name: "potato should be capitalized".into(),
                    ..Default::default()
                }
                .into(),
            }),
        )
        .unwrap();

        block_on(
            app_meta.repository.modify(Change::Create {
                thing: Place {
                    name: "Potato & Meat".into(),
                    subtype: PlaceType::Inn.into(),
                    ..Default::default()
                }
                .into(),
            }),
        )
        .unwrap();

        assert_autocomplete(
            &[
                ("Potato Johnson", "adult elf, they/them (unsaved)"),
                ("Potato & Meat", "inn (unsaved)"),
            ][..],
            StorageCommand::autocomplete("P", &app_meta),
        );

        assert!(StorageCommand::autocomplete("delete P", &app_meta).is_empty());

        assert_autocomplete(
            &[
                ("save Potato Johnson", "save character to journal"),
                (
                    "save potato should be capitalized",
                    "save character to journal",
                ),
                ("save Potato & Meat", "save place to journal"),
            ][..],
            StorageCommand::autocomplete("save ", &app_meta),
        );

        assert_autocomplete(
            &[
                ("load Potato Johnson", "adult elf, they/them (unsaved)"),
                ("load Potato & Meat", "inn (unsaved)"),
            ][..],
            StorageCommand::autocomplete("load P", &app_meta),
        );

        assert_autocomplete(
            &[("delete [name]", "remove an entry from journal")][..],
            StorageCommand::autocomplete("delete", &app_meta),
        );

        assert_autocomplete(
            &[("load [name]", "load an entry")][..],
            StorageCommand::autocomplete("load", &app_meta),
        );

        assert_autocomplete(
            &[("save [name]", "save an entry to journal")][..],
            StorageCommand::autocomplete("s", &app_meta),
        );

        assert_autocomplete(
            &[("journal", "list journal contents")][..],
            StorageCommand::autocomplete("j", &app_meta),
        );

        assert!(StorageCommand::autocomplete("p", &app_meta).is_empty());

        assert_autocomplete(
            &[("Potato Johnson", "adult elf, they/them (unsaved)")][..],
            StorageCommand::autocomplete("Potato Johnson", &app_meta),
        );
    }

    #[test]
    fn display_test() {
        let app_meta = AppMeta::new(NullDataStore::default());

        vec![
            StorageCommand::Change {
                change: Change::Delete {
                    id: "Potato Johnson".into(),
                    name: "Potato Johnson".to_string(),
                },
            },
            StorageCommand::Change {
                change: Change::Save {
                    name: "Potato Johnson".to_string(),
                },
            },
            StorageCommand::Journal,
            StorageCommand::Load {
                name: "Potato Johnson".to_string(),
            },
        ]
        .drain(..)
        .for_each(|command| {
            let command_string = command.to_string();
            assert_ne!("", command_string);
            assert_eq!(
                (Some(command), Vec::new()),
                StorageCommand::parse_input(&command_string, &app_meta),
                "{}",
                command_string,
            );
        });
    }

    fn assert_autocomplete(
        expected_suggestions: &[(&str, &str)],
        actual_suggestions: Vec<(String, String)>,
    ) {
        let mut expected: Vec<_> = expected_suggestions
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        expected.sort();

        let mut actual = actual_suggestions;
        actual.sort();

        assert_eq!(expected, actual);
    }
}
