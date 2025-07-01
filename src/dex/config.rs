use serde::Serialize;

use crate::dex::HTTP_PORT;

#[derive(Serialize)]
pub struct Storage {
    #[serde(rename = "type")]
    storage_type: String,
    config: StorageConfig,
}

impl Storage {
    pub fn sqlite() -> Self {
        Self {
            storage_type: String::from("sqlite3"),
            config: StorageConfig {
                file: String::from("/etc/dex/dex.db"),
            },
        }
    }
}

#[derive(Serialize)]
pub struct StorageConfig {
    file: String,
}

/// See https://dexidp.io/docs/configuration/
#[derive(Serialize)]
pub struct Config {
    pub issuer: String,
    pub storage: Storage,
    pub web: Web,
    #[serde(rename = "staticClients")]
    pub static_clients: Vec<PrivateClient>,
    #[serde(rename = "enablePasswordDB")]
    pub enable_password_db: bool,
    #[serde(rename = "staticPasswords")]
    pub static_passwords: Vec<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth2: Option<OAuth2>,
}

#[derive(Serialize)]
pub struct Web {
    pub http: String,
}

impl Web {
    pub fn http() -> Self {
        Self {
            http: format!(":{HTTP_PORT}"),
        }
    }
}

#[derive(Serialize)]
pub struct OAuth2 {
    #[serde(rename = "passwordConnector")]
    password_connector: String,
}

impl OAuth2 {
    pub fn allow_password_grant() -> Self {
        Self {
            password_connector: String::from("local"),
        }
    }
}

/// Definition of an OpenID client application.
#[derive(Serialize, Clone)]
pub struct PrivateClient {
    /// Unique identifier of the client.
    pub id: String,
    /// Display name of the client
    pub name: String,
    /// Allowed redirect
    #[serde(rename = "redirectURIs")]
    pub redirect_uris: Vec<String>,
    /// Cleartext secret that the client application authenticates against Dex with.
    pub secret: String,
}

impl PrivateClient {
    /// Creates a preconfigured client.
    pub fn simple_client() -> Self {
        Self {
            id: String::from("client"),
            name: String::from("Client"),
            redirect_uris: vec![String::from("http://localhost/oidc-callback")],
            secret: String::from("secret"),
        }
    }
}

/// Definition of a user that can authenticate in dex.
#[derive(Serialize, Clone)]
pub struct User {
    /// E-Mail address of the user. This is what the user enters into the login prompt.
    ///
    /// You can – i.e. – use example.org to avoid specifying a "real" address here. Dex will *not* send
    /// e-mails.
    pub email: String,

    /// Generate the hash of "password" with
    /// ```sh
    /// echo password | htpasswd -BinC 10 admin | cut -d: -f2
    /// ```
    pub hash: String,

    /// Display name of the user.
    pub username: String,

    /// Unique identifier (subject/sub) for the user.
    #[serde(rename = "userID")]
    pub user_id: String,
}

impl User {
    /// Creates a user with the password "user"
    pub fn simple_user() -> Self {
        Self {
            email: String::from("user@example.org"),
            username: String::from("User"),
            hash: String::from("$2y$10$l.yOBo5a8m1TnfVuvj/gX.y3vvnHiQs0G59rEwIVU2blgcqkUDLjS"),
            user_id: String::from("user"),
        }
    }
}
