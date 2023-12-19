use crate::SYSTEM_MESSAGE;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;
use std::{error, fmt, io};

#[derive(PartialEq)]
pub enum Role {
    ASSISTANT,
    USER,
    SYSTEM,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Role::ASSISTANT => write!(f, "assistant"),
            Role::USER => write!(f, "user"),
            Role::SYSTEM => write!(f, "system"),
        }
    }
}

impl FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "assistant" => Ok(Role::ASSISTANT),
            "user" => Ok(Role::USER),
            "system" => Ok(Role::SYSTEM),
            _ => Err(()),
        }
    }
}

impl<'a> From<&'a str> for Role {
    fn from(s: &'a str) -> Self {
        Role::from_str(s).unwrap_or_else(|_| panic!("Invalid role: {}", s))
    }
}

impl Serialize for Role {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

struct RoleVisitor;

impl<'de> Visitor<'de> for RoleVisitor {
    type Value = Role;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string representing a role")
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        Role::from_str(v).map_err(|_| {
            serde::de::Error::custom(format!("Unable to deserialize string {} to Role", v))
        })
    }
}

impl<'de> Deserialize<'de> for Role {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(RoleVisitor)
    }
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub(crate) role: Role,
    pub(crate) content: String,
}

impl Message {
    pub fn new(role: Role, content: &str) -> Self {
        Self {
            role,
            content: content.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct History {
    pub(crate) id: uuid::Uuid,
    pub(crate) history: Vec<Message>,
}

impl History {
    pub fn new() -> Self {
        let id = uuid::Uuid::new_v4();
        Self {
            id,
            history: vec![Message {
                role: Role::SYSTEM,
                content: SYSTEM_MESSAGE.to_string(),
            }],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MyError {
    message: String,
}

impl From<io::Error> for MyError {
    fn from(error: io::Error) -> Self {
        MyError {
            message: error.to_string(),
        }
    }
}

impl From<serde_json::Error> for MyError {
    fn from(error: serde_json::Error) -> Self {
        MyError {
            message: error.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ChatMessageBuildError(&'static str);

impl fmt::Display for ChatMessageBuildError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl error::Error for ChatMessageBuildError {}

impl From<&'static str> for ChatMessageBuildError {
    fn from(s: &'static str) -> Self {
        ChatMessageBuildError(s)
    }
}
