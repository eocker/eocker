use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone)]
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
        let sp = s
            .split_once(":")
            .ok_or_else(|| Error::custom("could not split digest"))?;
        let algorithm = sp.0.to_string();
        let hex = sp.1.to_string();
        // ... checks for validity of digest segments ...
        Ok(Hash {
            algorithm: algorithm,
            hex: hex,
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
