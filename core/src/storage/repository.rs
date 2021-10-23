use crate::storage::DataStore;
use crate::time::Time;
use crate::{Thing, Uuid};
use std::collections::{HashMap, VecDeque};
use std::fmt;

const RECENT_MAX_LEN: usize = 100;

pub struct Repository {
    cache: HashMap<Uuid, Thing>,
    data_store: Box<dyn DataStore>,
    data_store_enabled: bool,
    recent: VecDeque<Thing>,
    time: Time,
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Change {
    /// Create a new thing and store it in recent entries.
    ///
    /// Reverse: Delete { id: Id::Name }
    Create { thing: Thing },

    /// Create a new thing and store it in the journal.
    ///
    /// Reverse: Delete { id: Id::Uuid }
    CreateAndSave { thing: Thing },

    /// Delete a thing from recent or journal.
    ///
    /// Reverse: Create (recent) or CreateAndSave (journal)
    Delete { id: Id },

    /// Transfer a thing from recent to journal.
    ///
    /// Reverse: Unsave
    Save { name: String },

    /// Transfer a thing from journal to recent. Only triggerable as the reverse to Save.
    ///
    /// Reverse: Save
    Unsave { uuid: Uuid },
}

#[derive(Clone, Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub enum Id {
    Name(String),
    Uuid(Uuid),
}

#[derive(Debug, PartialEq)]
pub enum Error {
    DataStoreFailed,
    MissingName,
    NameAlreadyExists,
    NotFound,
}

impl Repository {
    pub fn new(data_store: impl DataStore + 'static) -> Self {
        Self {
            cache: HashMap::default(),
            data_store: Box::new(data_store),
            data_store_enabled: false,
            recent: VecDeque::default(),
            time: Time::try_new(1, 8, 0, 0).unwrap(),
        }
    }

    pub async fn init(&mut self) {
        let things = self.data_store.get_all_the_things().await;

        if let Ok(mut things) = things {
            self.cache = things
                .drain(..)
                .filter_map(|thing| {
                    if let Some(&uuid) = thing.uuid() {
                        Some((uuid, thing))
                    } else {
                        None
                    }
                })
                .collect();
            self.data_store_enabled = true;
        }

        if let Ok(Some(time_str)) = self.data_store.get_value("time").await {
            if let Ok(time) = time_str.parse() {
                self.set_time(time).await;
            }
        }
    }

    pub fn load(&self, id: &Id) -> Option<&Thing> {
        match id {
            Id::Name(name) => self.load_thing_by_name(name),
            Id::Uuid(uuid) => self.cache.get(uuid),
        }
    }

    pub fn all(&self) -> impl Iterator<Item = &Thing> {
        self.journal().chain(self.recent())
    }

    pub fn recent(&self) -> impl Iterator<Item = &Thing> {
        self.recent.as_slices().0.iter()
    }

    pub fn journal(&self) -> impl Iterator<Item = &Thing> {
        self.cache.values()
    }

    pub async fn modify(&mut self, change: Change) -> Result<Change, (Change, Error)> {
        match change {
            Change::Create { thing } => self
                .create_thing(thing)
                .map(|name| Change::Delete {
                    id: name.as_str().into(),
                })
                .map_err(|(thing, e)| (Change::Create { thing }, e)),
            Change::CreateAndSave { thing } => self
                .create_and_save_thing(thing)
                .await
                .map(|uuid| Change::Delete { id: uuid.into() })
                .map_err(|(thing, e)| (Change::CreateAndSave { thing }, e)),
            Change::Delete { id } => match id {
                Id::Name(name) => self
                    .delete_thing_by_name(&name)
                    .await
                    .map(|thing| {
                        if thing.uuid().is_some() {
                            Change::CreateAndSave { thing }
                        } else {
                            Change::Create { thing }
                        }
                    })
                    .map_err(|e| (Change::Delete { id: Id::Name(name) }, e)),
                Id::Uuid(uuid) => self
                    .delete_thing_by_uuid(&uuid)
                    .await
                    .map(|thing| Change::CreateAndSave { thing })
                    .map_err(|e| (Change::Delete { id: Id::Uuid(uuid) }, e)),
            },
            Change::Save { name } => self
                .save_thing_by_name(&name.to_lowercase())
                .await
                .map(|uuid| Change::Unsave { uuid })
                .map_err(|e| (Change::Save { name }, e)),
            Change::Unsave { uuid } => self
                .unsave_thing_by_uuid(&uuid)
                .await
                .map(|name| Change::Save { name })
                .map_err(|e| (Change::Unsave { uuid }, e)),
        }
    }

    pub async fn set_time(&mut self, time: Time) {
        self.data_store
            .set_value("time", &time.display_short().to_string())
            .await
            .ok();
        self.time = time;
    }

    pub fn get_time(&self) -> &Time {
        &self.time
    }

    pub fn data_store_enabled(&self) -> bool {
        self.data_store_enabled
    }

    fn push_recent(&mut self, thing: Thing) {
        while self.recent.len() >= RECENT_MAX_LEN {
            self.recent.pop_front();
        }

        self.recent.push_back(thing);
    }

    fn take_recent<F>(&mut self, f: F) -> Option<Thing>
    where
        F: Fn(&Thing) -> bool,
    {
        if let Some(index) =
            self.recent
                .iter()
                .enumerate()
                .find_map(|(i, t)| if f(t) { Some(i) } else { None })
        {
            self.recent.remove(index)
        } else {
            None
        }
    }

    fn create_thing(&mut self, thing: Thing) -> Result<String, (Thing, Error)> {
        if let Some(name) = thing.name().value() {
            if self.load_thing_by_name(&name.to_lowercase()).is_some() {
                Err((thing, Error::NameAlreadyExists))
            } else {
                let name = name.to_string();
                self.push_recent(thing);
                Ok(name)
            }
        } else {
            Err((thing, Error::MissingName))
        }
    }

    async fn create_and_save_thing(&mut self, thing: Thing) -> Result<Uuid, (Thing, Error)> {
        if let Some(name) = thing.name().value() {
            if self.load_thing_by_name(&name.to_lowercase()).is_some() {
                Err((thing, Error::NameAlreadyExists))
            } else {
                self.save_thing(thing).await
            }
        } else {
            Err((thing, Error::MissingName))
        }
    }

    async fn delete_thing_by_name(&mut self, name: &str) -> Result<Thing, Error> {
        let name_matches = |s: &String| s.to_lowercase() == name;

        let cached_uuid = if let Some((uuid, _)) = self
            .cache
            .iter()
            .find(|(_, t)| t.name().value().map_or(false, name_matches))
        {
            Some(*uuid)
        } else {
            None
        };

        if let Some(uuid) = cached_uuid {
            self.delete_thing_by_uuid(&uuid).await
        } else if let Some(thing) =
            self.take_recent(|t| t.name().value().map_or(false, name_matches))
        {
            Ok(thing)
        } else {
            Err(Error::NotFound)
        }
    }

    async fn delete_thing_by_uuid(&mut self, uuid: &Uuid) -> Result<Thing, Error> {
        match (
            self.cache.remove(uuid),
            self.data_store.delete_thing_by_uuid(uuid).await,
        ) {
            (Some(thing), Ok(())) => Ok(thing),
            (Some(_), Err(())) => Err(Error::DataStoreFailed),
            (None, _) => Err(Error::NotFound),
        }
    }

    fn load_thing_by_name<'a>(&'a self, name: &str) -> Option<&'a Thing> {
        self.all()
            .find(|t| t.name().value().map_or(false, |s| s.to_lowercase() == name))
    }

    async fn save_thing_by_name(&mut self, name: &str) -> Result<Uuid, Error> {
        if let Some(thing) =
            self.take_recent(|t| t.name().value().map_or(false, |s| s.to_lowercase() == name))
        {
            self.save_thing(thing).await.map_err(|(thing, e)| {
                self.push_recent(thing);
                e
            })
        } else {
            Err(Error::NotFound)
        }
    }

    async fn save_thing(&mut self, mut thing: Thing) -> Result<Uuid, (Thing, Error)> {
        let uuid = Uuid::new_v4();
        thing.set_uuid(uuid);

        match self.data_store.save_thing(&thing).await {
            Ok(()) => {
                self.cache.insert(uuid, thing);
                Ok(uuid)
            }
            Err(()) => {
                thing.clear_uuid();
                Err((thing, Error::DataStoreFailed))
            }
        }
    }

    async fn unsave_thing_by_uuid(&mut self, uuid: &Uuid) -> Result<String, Error> {
        let mut thing = self.delete_thing_by_uuid(uuid).await?;
        thing.clear_uuid();
        self.create_thing(thing).map_err(|(_, e)| e)
    }
}

impl From<Uuid> for Id {
    fn from(input: Uuid) -> Self {
        Id::Uuid(input)
    }
}

impl From<&String> for Id {
    fn from(input: &String) -> Self {
        input.as_str().into()
    }
}

impl From<&str> for Id {
    fn from(input: &str) -> Self {
        Id::Name(input.to_lowercase())
    }
}

impl fmt::Debug for Repository {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Repository {{ cache: {:?}, data_store_enabled: {:?}, recent: {:?}, time: {:?} }}",
            self.cache, self.data_store_enabled, self.recent, self.time,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::storage::data_store::{MemoryDataStore, NullDataStore};
    use crate::world::{Location, Npc};
    use tokio_test::block_on;

    const TEST_UUID: Uuid = Uuid::from_u128(u128::MAX);

    #[test]
    fn recent_test() {
        let mut repository = empty_repo();

        (0..RECENT_MAX_LEN).for_each(|i| {
            repository.push_recent(
                Npc {
                    name: format!("Thing {}", i).into(),
                    ..Default::default()
                }
                .into(),
            );
            assert_eq!(i + 1, repository.recent.len());
        });

        assert_eq!(
            Some(&"Thing 0".to_string()),
            repository
                .recent()
                .next()
                .and_then(|thing| thing.name().value()),
        );

        repository.push_recent(
            Npc {
                name: "The Cat in the Hat".into(),
                ..Default::default()
            }
            .into(),
        );
        assert_eq!(RECENT_MAX_LEN, repository.recent.len());

        assert_eq!(
            Some(&"Thing 1".to_string()),
            repository
                .recent()
                .next()
                .and_then(|thing| thing.name().value()),
        );

        assert_eq!(
            Some(&"The Cat in the Hat".to_string()),
            repository
                .recent()
                .last()
                .and_then(|thing| thing.name().value()),
        );
    }

    #[test]
    fn all_journal_test() {
        let repo = repo();
        assert_eq!(1, repo.recent().count());
        assert_eq!(1, repo.journal().count());
        assert_eq!(2, repo.all().count());
    }

    #[test]
    fn load_test_from_recent_by_name() {
        assert_eq!(
            "Odysseus",
            repo()
                .load(&"ODYSSEUS".into())
                .and_then(|thing| thing.name().value())
                .unwrap(),
        );
    }

    #[test]
    fn load_test_from_journal_by_name() {
        assert_eq!(
            "Olympus",
            repo()
                .load(&"OLYMPUS".into())
                .and_then(|thing| thing.name().value())
                .unwrap(),
        );
    }

    #[test]
    fn load_test_not_found() {
        assert!(repo().load(&"NOBODY".into()).is_none());
    }

    #[test]
    fn load_test_by_uuid() {
        assert_eq!(
            "Olympus",
            repo()
                .load(&TEST_UUID.into())
                .and_then(|thing| thing.name().value())
                .unwrap(),
        );
    }

    #[test]
    fn change_test_delete_from_journal_by_name() {
        let (mut repo, data_store) = repo_data_store();
        assert_eq!(
            Ok(Change::CreateAndSave {
                thing: Location {
                    uuid: Some(TEST_UUID.into()),
                    name: "Olympus".into(),
                    ..Default::default()
                }
                .into()
            }),
            block_on(repo.modify(Change::Delete {
                id: "olympus".into(),
            })),
        );
        assert_eq!(0, repo.journal().count());
        assert_eq!(0, block_on(data_store.get_all_the_things()).unwrap().len());
    }

    #[test]
    fn change_test_delete_from_recent_by_name() {
        let mut repo = repo();
        assert_eq!(
            Ok(Change::Create {
                thing: Npc {
                    name: "Odysseus".into(),
                    ..Default::default()
                }
                .into()
            }),
            block_on(repo.modify(Change::Delete {
                id: "ODYSSEUS".into(),
            })),
        );
        assert_eq!(0, repo.recent().count());
    }

    #[test]
    fn change_test_delete_by_name_not_found() {
        let change = Change::Delete {
            id: "NOBODY".into(),
        };

        assert_eq!(
            block_on(repo().modify(change.clone())),
            Err((change, Error::NotFound)),
        );
    }

    #[test]
    fn change_test_delete_by_uuid_success() {
        let (mut repo, data_store) = repo_data_store();
        let change = Change::Delete {
            id: TEST_UUID.into(),
        };

        let result = block_on(repo.modify(change)).unwrap();

        assert_eq!(
            Change::CreateAndSave {
                thing: Location {
                    uuid: Some(TEST_UUID.into()),
                    name: "Olympus".into(),
                    ..Default::default()
                }
                .into()
            },
            result,
        );
        assert_eq!(0, repo.journal().count());
        assert_eq!(0, block_on(data_store.get_all_the_things()).unwrap().len());
    }

    #[test]
    fn change_test_delete_by_uuid_not_found() {
        let change = Change::Delete {
            id: Uuid::nil().into(),
        };

        let result = block_on(repo().modify(change.clone())).unwrap_err();

        assert_eq!((change, Error::NotFound), result);
    }

    #[test]
    fn change_test_delete_by_uuid_data_store_failed() {
        let (mut repo, mut data_store) = repo_data_store();
        block_on(data_store.delete_thing_by_uuid(&TEST_UUID)).unwrap();
        let change = Change::Delete {
            id: TEST_UUID.into(),
        };

        let result = block_on(repo.modify(change.clone())).unwrap_err();

        assert_eq!((change, Error::DataStoreFailed), result);
    }

    #[test]
    fn change_test_create_success() {
        let mut repo = empty_repo();
        assert_eq!(
            Ok(Change::Delete {
                id: "Odysseus".into()
            }),
            block_on(
                repo.modify(Change::Create {
                    thing: Npc {
                        name: "Odysseus".into(),
                        ..Default::default()
                    }
                    .clone()
                    .into()
                })
            ),
        );
        assert_eq!(1, repo.recent().count());
    }

    #[test]
    fn change_test_create_already_exists_in_journal() {
        let (mut repo, data_store) = repo_data_store();
        let change = Change::Create {
            thing: Npc {
                name: "Olympus".into(),
                ..Default::default()
            }
            .clone()
            .into(),
        };

        assert_eq!(
            block_on(repo.modify(change.clone())),
            Err((change, Error::NameAlreadyExists)),
        );
        assert_eq!(1, repo.journal().count());
        assert_eq!(1, block_on(data_store.get_all_the_things()).unwrap().len());
    }

    #[test]
    fn change_test_create_already_exists_in_recent() {
        let mut repo = repo();
        let change = Change::Create {
            thing: Location {
                name: "Odysseus".into(),
                ..Default::default()
            }
            .clone()
            .into(),
        };

        assert_eq!(
            block_on(repo.modify(change.clone())),
            Err((change, Error::NameAlreadyExists)),
        );
        assert_eq!(1, repo.recent().count());
    }

    #[test]
    fn change_test_save_success() {
        let (mut repo, data_store) = repo_data_store();

        assert_eq!(1, repo.journal().count());
        assert_eq!(1, repo.recent().count());

        assert!(matches!(
            block_on(repo.modify(Change::Save {
                name: "ODYSSEUS".to_string()
            })),
            Ok(Change::Unsave { .. })
        ));

        assert_eq!(2, repo.journal().count());
        assert_eq!(2, block_on(data_store.get_all_the_things()).unwrap().len());
        assert_eq!(0, repo.recent().count());
    }

    #[test]
    fn change_test_save_data_store_failed() {
        let mut repo = null_repo();

        block_on(
            repo.modify(Change::Create {
                thing: Location {
                    name: "Odysseus".into(),
                    ..Default::default()
                }
                .into(),
            }),
        )
        .unwrap();

        assert_eq!(0, repo.journal().count());
        assert_eq!(1, repo.recent().count());

        let change = Change::Save {
            name: "ODYSSEUS".to_string(),
        };
        assert_eq!(
            block_on(repo.modify(change.clone())),
            Err((change, Error::DataStoreFailed)),
        );

        assert_eq!(0, repo.journal().count());
        assert_eq!(1, repo.recent().count());
    }

    #[test]
    fn change_test_save_already_saved() {
        let mut repo = repo();

        let change = Change::Save {
            name: "OLYMPUS".to_string(),
        };
        assert_eq!(
            block_on(repo.modify(change.clone())),
            Err((change, Error::NotFound)),
        );
    }

    #[test]
    fn change_test_save_not_found() {
        let mut repo = repo();
        let change = Change::Save {
            name: "NOBODY".to_string(),
        };

        assert_eq!(
            block_on(repo.modify(change.clone())),
            Err((change, Error::NotFound)),
        );
    }

    #[test]
    fn change_test_create_and_save_success() {
        let (mut repo, data_store) = empty_repo_data_store();
        assert!(matches!(
            block_on(
                repo.modify(Change::CreateAndSave {
                    thing: Npc {
                        name: "Odysseus".into(),
                        ..Default::default()
                    }
                    .clone()
                    .into()
                })
            ),
            Ok(Change::Delete {
                id: Id::Uuid { .. }
            }),
        ));
        assert_eq!(1, repo.journal().count());
        assert_eq!(
            repo.journal().next().unwrap().uuid().unwrap(),
            block_on(data_store.get_all_the_things())
                .unwrap()
                .first()
                .unwrap()
                .uuid()
                .unwrap(),
        );
    }

    #[test]
    fn change_test_create_and_save_already_exists_in_journal() {
        let (mut repo, data_store) = repo_data_store();
        let change = Change::CreateAndSave {
            thing: Location {
                name: "Odysseus".into(),
                ..Default::default()
            }
            .clone()
            .into(),
        };

        assert_eq!(
            block_on(repo.modify(change.clone())),
            Err((change, Error::NameAlreadyExists)),
        );
        assert_eq!(1, repo.journal().count());
        assert_eq!(1, block_on(data_store.get_all_the_things()).unwrap().len());
    }

    #[test]
    fn change_test_create_and_save_already_exists_in_recent() {
        let (mut repo, data_store) = repo_data_store();
        let change = Change::CreateAndSave {
            thing: Npc {
                name: "Olympus".into(),
                ..Default::default()
            }
            .clone()
            .into(),
        };

        assert_eq!(
            block_on(repo.modify(change.clone())),
            Err((change, Error::NameAlreadyExists)),
        );
        assert_eq!(1, repo.journal().count());
        assert_eq!(1, block_on(data_store.get_all_the_things()).unwrap().len());
    }

    #[test]
    fn change_test_create_and_save_data_store_failed() {
        let mut repo = null_repo();

        let change = Change::CreateAndSave {
            thing: Location {
                name: "Odysseus".into(),
                ..Default::default()
            }
            .into(),
        };

        assert_eq!(
            block_on(repo.modify(change.clone())),
            Err((change, Error::DataStoreFailed)),
        );
        assert_eq!(0, repo.journal().count());
    }

    #[test]
    fn debug_test() {
        assert_eq!(
            "Repository { cache: {}, data_store_enabled: false, recent: [], time: Time { days: 1, hours: 8, minutes: 0, seconds: 0 } }",
            format!("{:?}", empty_repo()),
        );
    }

    #[test]
    fn data_store_enabled_test_success() {
        let mut repo = repo();
        block_on(repo.init());
        assert_eq!(true, repo.data_store_enabled());
    }

    #[test]
    fn data_store_enabled_test_failure() {
        let mut repo = null_repo();
        block_on(repo.init());
        assert_eq!(false, repo.data_store_enabled());
    }

    fn repo() -> Repository {
        repo_data_store().0
    }

    fn repo_data_store() -> (Repository, MemoryDataStore) {
        let mut data_store = MemoryDataStore::default();
        block_on(
            data_store.save_thing(
                &Location {
                    uuid: Some(TEST_UUID.into()),
                    name: "Olympus".into(),
                    ..Default::default()
                }
                .into(),
            ),
        )
        .unwrap();

        let mut repo = Repository::new(data_store.clone());
        block_on(repo.init());

        repo.recent.push_back(
            Npc {
                name: "Odysseus".into(),
                ..Default::default()
            }
            .into(),
        );

        (repo, data_store)
    }

    fn empty_repo() -> Repository {
        Repository::new(MemoryDataStore::default())
    }

    fn empty_repo_data_store() -> (Repository, MemoryDataStore) {
        let data_store = MemoryDataStore::default();
        (Repository::new(data_store.clone()), data_store)
    }

    fn null_repo() -> Repository {
        Repository::new(NullDataStore::default())
    }
}
