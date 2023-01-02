use serenity::prelude::*;

pub struct AuthData {
    pub giphy: String,
    pub discord: String,
}

impl TypeMapKey for AuthData {
    type Value = AuthData;
}
