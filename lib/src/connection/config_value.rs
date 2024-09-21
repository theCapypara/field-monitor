use secure_string::SecureString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValue {
    SecureString(SecureString),
    SerdeValue(serde_yaml::Value),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigValueRef<'a> {
    SecureString(&'a SecureString),
    SerdeValue(&'a serde_yaml::Value),
}

impl ConfigValue {
    pub fn as_serde_value(&self) -> Option<&serde_yaml::Value> {
        match self {
            Self::SerdeValue(val) => Some(val),
            _ => None,
        }
    }
}

impl<'a> ConfigValueRef<'a> {
    pub fn as_serde_value(self) -> Option<&'a serde_yaml::Value> {
        match self {
            Self::SerdeValue(val) => Some(val),
            _ => None,
        }
    }
}

impl From<serde_yaml::Value> for ConfigValue {
    fn from(value: serde_yaml::Value) -> Self {
        Self::SerdeValue(value)
    }
}

impl From<SecureString> for ConfigValue {
    fn from(value: SecureString) -> Self {
        Self::SecureString(value)
    }
}

impl<'a> From<&'a ConfigValue> for ConfigValueRef<'a> {
    fn from(value: &'a ConfigValue) -> Self {
        match value {
            ConfigValue::SecureString(v) => Self::SecureString(v),
            ConfigValue::SerdeValue(v) => Self::SerdeValue(v),
        }
    }
}
