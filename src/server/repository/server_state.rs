use super::journeys_repository::JourneysRepository;
use super::users_repository::UsersRepository;
use sqlx::{Pool, Sqlite};

pub struct ServerState {
    pub journeys_repo: JourneysRepository,
    pub users_repo: UsersRepository,
}

impl ServerState {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self {
            journeys_repo: JourneysRepository::new(pool.clone()),
            users_repo: UsersRepository::new(pool),
        }
    }
}