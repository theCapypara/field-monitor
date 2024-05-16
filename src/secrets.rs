use std::ops::Deref;

use gettextrs::gettext;
use log::warn;

use crate::config::APP_ID;

#[derive(Debug)]
pub struct SecretManager {
    keyring: oo7::portal::Keyring,
}

impl SecretManager {
    pub async fn new() -> anyhow::Result<Self> {
        let keyring = oo7::portal::Keyring::load_default().await?;
        Ok(Self { keyring })
    }

    pub async fn lookup(&self, connection_id: &str, field: &str) -> anyhow::Result<Option<String>> {
        let mut attributes = std::collections::HashMap::new();
        attributes.insert("app", APP_ID);
        attributes.insert("connection_id", connection_id);
        attributes.insert("field", field);

        let items = self
            .keyring
            .search_items(&attributes)
            .await
            .inspect_err(|err| {
                warn!("failed to lookup a secret for {connection_id}/{field}: {err}")
            })?;

        match items.first() {
            None => Ok(None),
            Some(item) => {
                let secret_raw = item.secret();
                let secret = String::from_utf8(secret_raw.deref().clone())?;
                Ok(Some(secret))
            }
        }
    }

    pub async fn store(
        &self,
        connection_id: &str,
        field: &str,
        password: &str,
    ) -> anyhow::Result<()> {
        let mut attributes = std::collections::HashMap::new();
        attributes.insert("app", APP_ID);
        attributes.insert("connection_id", connection_id);
        attributes.insert("field", field);

        self.keyring
            .create_item(
                &gettext("A secret value used by Field Monitor"),
                &attributes,
                password,
                true,
            )
            .await
            .inspect_err(|err| warn!("failed to store a secret for {connection_id}/{field}: {err}"))
            .map_err(Into::into)
            .map(drop)
    }

    pub async fn clear(&self, connection_id: &str, field: &str) -> anyhow::Result<()> {
        let mut attributes = std::collections::HashMap::new();
        attributes.insert("app", APP_ID);
        attributes.insert("connection_id", connection_id);
        attributes.insert("field", field);

        self.keyring
            .delete(&attributes)
            .await
            .inspect_err(|err| warn!("failed to clear a secret for {connection_id}/{field}: {err}"))
            .map_err(Into::into)
    }
}
