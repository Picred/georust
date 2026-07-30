use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};

pub struct Authenticator {}

impl Authenticator {
    fn generate_salt() -> SaltString {
        SaltString::generate(&mut OsRng)
    }
    pub fn encrypt_password(password: &[u8]) -> Result<String, sqlx::Error> {
        let salt = Authenticator::generate_salt();

        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password, &salt)
            .map_err(|e| sqlx::Error::Protocol(e.to_string()))?
            .to_string();

        Ok(password_hash)
    }

    pub fn verify_password(
        input_password: &[u8],
        stored_password: String,
    ) -> Result<bool, sqlx::Error> {
        let parsed_hash = match PasswordHash::new(&stored_password) {
            Ok(hash) => hash,
            Err(_) => return Ok(false), // Hash nel DB corrotto o non valido
        };

        let argon2 = Argon2::default();
        let is_valid_password = argon2.verify_password(input_password, &parsed_hash).is_ok();

        Ok(is_valid_password)
    }
}
