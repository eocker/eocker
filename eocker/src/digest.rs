use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug)]
pub struct Hash {
    pub algorithm: String,
    pub hex: String,
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D>(deserializer: D) -> Result<Hash, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;
        let mut sp = s.split(":");
        // TODO(hasheddan): perform more robust checks
        Ok(Hash {
            algorithm: sp.next().unwrap().to_string(),
            hex: sp.next().unwrap().to_string(),
        })
    }
}

impl Serialize for Hash {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(format!("{}:{}", self.algorithm, self.hex).as_str())
    }
}

