use std::{borrow::Cow, collections::HashMap};

use parse_display::{Display, FromStr};
use testcontainers::{
    core::{ContainerPort, WaitFor},
    Image,
};

const NAME: &str = "bitnami/openldap";
const TAG: &str = "2.6.8";
const OPENLDAP_PORT: ContainerPort = ContainerPort::Tcp(1389);

/// Module to work with [`OpenLDAP`] inside of tests.
///
/// Starts an instance of OpenLDAP.
/// This module is based on the [`bitnami/openldap docker image`].
/// See the [`OpenLDAP configuration guide`] for further configuration options.
///
/// # Example
/// ```
/// use testcontainers_modules::{openldap, testcontainers::runners::SyncRunner};
///
/// let openldap_instance = openldap::OpenLDAP::default().start().unwrap();
/// let connection_string = format!(
///     "ldap://{}:{}",
///     openldap_instance.get_host().unwrap(),
///     openldap_instance.get_host_port_ipv4(1389).unwrap(),
/// );
/// let mut conn = ldap3::LdapConn::new(&connection_string).unwrap();
/// let ldap3::SearchResult(rs, _) = conn
///     .search(
///         "ou=users,dc=example,dc=org",
///         ldap3::Scope::Subtree,
///         "(cn=ma*)",
///         vec!["cn"],
///     )
///     .unwrap();
/// let results: Vec<_> = rs.into_iter().map(ldap3::SearchEntry::construct).collect();
/// assert_eq!(results.len(), 0);
/// ```
///
/// [`OpenLDAP`]: https://www.openldap.org/
/// [`bitnami/openldap docker image`]: https://hub.docker.com/r/bitnami/openldap
/// [`OpenLDAP configuration guide`]: https://www.openldap.org/doc/admin26/guide.html
#[derive(Debug, Clone)]
pub struct OpenLDAP {
    env_vars: HashMap<String, String>,
    users: Vec<User>,
}
#[derive(Debug, Clone)]
struct User {
    username: String,
    password: String,
}

impl OpenLDAP {
    /// Sets the LDAP baseDN (or suffix) of the LDAP tree.
    /// Default: `"dc=example,dc=org"`
    pub fn with_base_dn(mut self, base_dn: impl ToString) -> Self {
        self.env_vars
            .insert("LDAP_ROOT".to_owned(), base_dn.to_string());
        self
    }
    /// Sets an admin account (there can only be one)
    /// Default username: `"admin"` => dn: `cn=admin,dc=example,dc=org` if using the default `base_dn` instead of overriding via [`OpenLDAP::with_base_dn`].
    /// Default password: `"adminpassword"`
    pub fn with_admin(mut self, username: impl ToString, password: impl ToString) -> Self {
        self.env_vars
            .insert("LDAP_ADMIN_USERNAME".to_owned(), username.to_string());
        self.env_vars
            .insert("LDAP_ADMIN_PASSWORD".to_owned(), password.to_string());
        self
    }

    /// Sets a configuration admin user (there can only be one)
    /// Default: `None`
    pub fn with_config_admin(mut self, username: impl ToString, password: impl ToString) -> Self {
        self.env_vars
            .insert("LDAP_CONFIG_ADMIN_ENABLED".to_owned(), "yes".to_owned());
        self.env_vars.insert(
            "LDAP_CONFIG_ADMIN_USERNAME".to_owned(),
            username.to_string(),
        );
        self.env_vars.insert(
            "LDAP_CONFIG_ADMIN_PASSWORD".to_owned(),
            password.to_string(),
        );
        self
    }

    /// Sets an accesslog admin user up (there can only be one)
    /// Configuring the admin for the access log can be done via [`OpenLDAP::with_accesslog_admin`]
    /// Default: `None`
    pub fn with_accesslog_settings(
        mut self,
        AccesslogSettings {
            log_operations,
            log_success,
            log_purge,
            log_old,
            log_old_attribute,
        }: AccesslogSettings,
    ) -> Self {
        self.env_vars
            .insert("LDAP_ENABLE_ACCESSLOG".to_owned(), "yes".to_owned());
        self.env_vars.insert(
            "LDAP_ACCESSLOG_LOGOPS".to_owned(),
            log_operations.to_string(),
        );
        if log_success {
            self.env_vars
                .insert("LDAP_ACCESSLOG_LOGSUCCESS".to_owned(), "TRUE".to_owned());
        } else {
            self.env_vars
                .insert("LDAP_ACCESSLOG_LOGSUCCESS".to_owned(), "FALSE".to_owned());
        }
        self.env_vars.insert(
            "LDAP_ACCESSLOG_LOGPURGE".to_owned(),
            format!("{} {}", log_purge.0, log_purge.1),
        );
        self.env_vars
            .insert("LDAP_ACCESSLOG_LOGOLD".to_owned(), log_old.to_string());
        self.env_vars.insert(
            "LDAP_ACCESSLOG_LOGOLDATTR".to_owned(),
            log_old_attribute.to_string(),
        );
        self
    }

    /// Activates the access log and sets the admin user up (there can only be one)
    /// Configuring how [`OpenLDAP`] logs can be done via [`OpenLDAP::with_accesslog_settings`]
    /// Default: `None`
    pub fn with_accesslog_admin(
        mut self,
        username: impl ToString,
        password: impl ToString,
    ) -> Self {
        self.env_vars
            .insert("LDAP_ENABLE_ACCESSLOG".to_owned(), "yes".to_owned());
        self.env_vars.insert(
            "LDAP_ACCESSLOG_ADMIN_USERNAME".to_owned(),
            username.to_string(),
        );
        self.env_vars.insert(
            "LDAP_ACCESSLOG_ADMIN_PASSWORD".to_owned(),
            password.to_string(),
        );
        self
    }

    /// Adds a user (can be called multiple times)
    /// Default: `[]`
    ///
    /// Alternatively, users can be added via [`OpenLDAP::with_users`].
    pub fn with_user(mut self, username: impl ToString, password: impl ToString) -> Self {
        self.users.push(User {
            username: username.to_string(),
            password: password.to_string(),
        });
        self
    }

    /// Add users (can be called multiple times)
    /// Default: `[]`
    ///
    /// Alternatively, users can be added via [`OpenLDAP::with_user`].
    pub fn with_users<Username: ToString, Password: ToString>(
        mut self,
        user_password: impl IntoIterator<Item = (Username, Password)>,
    ) -> Self {
        for (username, password) in user_password.into_iter() {
            self.users.push(User {
                username: username.to_string(),
                password: password.to_string(),
            })
        }
        self
    }

    /// Sets the users' dc
    /// Default: `"users"`
    pub fn with_users_dc(mut self, user_dc: impl ToString) -> Self {
        self.env_vars
            .insert("LDAP_USER_DC".to_owned(), user_dc.to_string());
        self
    }

    /// Sets the users' group
    /// Default: `"readers"`
    pub fn with_users_group(mut self, users_group: impl ToString) -> Self {
        self.env_vars
            .insert("LDAP_GROUP".to_owned(), users_group.to_string());
        self
    }

    /// Extra schemas to add, among [`OpenLDAP`]'s distributed schemas.
    /// Default: `["cosine", "inetorgperson", "nis"]`
    pub fn with_extra_schemas<S: ToString>(
        mut self,
        extra_schemas: impl IntoIterator<Item = S>,
    ) -> Self {
        self.env_vars
            .insert("LDAP_ADD_SCHEMAS".to_owned(), "yes".to_owned());
        let extra_schemas = extra_schemas
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>()
            .join(", ");
        self.env_vars
            .insert("LDAP_EXTRA_SCHEMAS".to_owned(), extra_schemas);
        self
    }

    /// Allow anonymous bindings to the LDAP server.
    /// Default: `true`
    pub fn with_allow_anon_binding(mut self, allow_anon_binding: bool) -> Self {
        if allow_anon_binding {
            self.env_vars
                .insert("LDAP_ALLOW_ANON_BINDING".to_owned(), "yes".to_owned());
        } else {
            self.env_vars
                .insert("LDAP_ALLOW_ANON_BINDING".to_owned(), "no".to_owned());
        }
        self
    }

    /// Set hash to be used in generation of user passwords.
    /// Default: [`PasswordHash::SSHA`].
    pub fn with_ldap_password_hash(mut self, password_hash: PasswordHash) -> Self {
        self.env_vars
            .insert("LDAP_PASSWORD_HASH".to_owned(), password_hash.to_string());
        self
    }
}

/// hash to be used in generation of user passwords.
#[derive(Display, FromStr, Default, Debug, Clone, Copy, Eq, PartialEq)]
#[display("{{{}}}")] // it's escaped curly braces => `SSHA` gets displayed as `{SSHA}`
pub enum PasswordHash {
    #[default]
    /// [`PasswordHash::SHA`], but with a salt.
    /// It is believed to be the **most secure password storage scheme supported by slapd**.
    SSHA,
    /// Like the MD5 scheme, this simply feeds the password through an SHA hash process.
    ///
    /// <div class="warning">SHA is thought to be more secure than MD5, but the lack of salt leaves the scheme exposed to dictionary attacks.</div>
    SHA,
    /// [`PasswordHash::MD5`], but with a salt.
    /// Salt = Random data which means that there are many possible representations of a given plaintext password
    SMD5,
    /// Simply takes the MD5 hash of the password and stores it in base64 encoded form.
    /// <div class="warning">MD5 algorithm is fast, and because there is no salt the scheme is vulnerable to a dictionary attack</div>
    MD5,
    /// Uses the operating system's `crypt(3)` hash function.
    /// It normally produces the traditional Unix-style 13 character hash, but on systems with `glibc2` it can also generate the more secure 34-byte `MD5` hash.
    ///
    /// <div class="warning">
    ///
    /// This scheme uses the operating system's `crypt(3)` hash function.
    /// It is therefore operating system specific.
    ///
    /// </div>
    CRYPT,
    /// stored as-is
    CLEARTEXT,
}

/// Specifies which types of operations to log
#[derive(Display, FromStr, Default, Debug, Clone, Copy, Eq, PartialEq)]
#[display(style = "lowercase")]
pub enum AccesslogLogOperations {
    /// Logs only writes
    #[default]
    Writes,
    /// Logs only reads
    Reads,
    /// Logs only sessions
    Session,
    /// Logs everything
    All,
}

/// allows finer grained control of the access logging
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AccesslogSettings {
    log_operations: AccesslogLogOperations,
    log_success: bool,
    log_purge: (String, String),
    log_old: String,
    log_old_attribute: String,
}
impl AccesslogSettings {
    /// Specify which types of operations to log.
    /// Default: [`AccesslogLogOperations::Writes`]
    pub fn with_log_operations(mut self, log_operations: AccesslogLogOperations) -> Self {
        self.log_operations = log_operations;
        self
    }
    /// Whether successful operations should be logged
    /// Default: `true`
    pub fn with_log_success(mut self, log_success: bool) -> Self {
        self.log_success = log_success;
        self
    }
    /// When and how often old access log entries should be purged. Format "dd+hh:mm".
    /// Default: `("07+00:00", "01+00:00")`.
    pub fn with_log_purge(
        mut self,
        (when_log_purge, how_often_log_purge): (impl ToString, impl ToString),
    ) -> Self {
        self.log_purge = (when_log_purge.to_string(), how_often_log_purge.to_string());
        self
    }
    /// An LDAP filter that determines which entries should be logged.
    /// Default: `"(objectClass=*)"`
    pub fn with_log_old(mut self, log_old: impl ToString) -> Self {
        self.log_old = log_old.to_string();
        self
    }
    /// Specifies an attribute that should be logged.
    /// Default: `"objectClass"`.
    pub fn with_log_old_attribute(mut self, log_old_attribute: impl ToString) -> Self {
        self.log_old_attribute = log_old_attribute.to_string();
        self
    }
}

impl Default for AccesslogSettings {
    fn default() -> Self {
        Self {
            log_operations: AccesslogLogOperations::Writes,
            log_success: true,
            log_purge: ("07+00:00".to_owned(), "01+00:00".to_owned()),
            log_old: "(objectClass=*)".to_owned(),
            log_old_attribute: "objectClass".to_owned(),
        }
    }
}

impl Default for OpenLDAP {
    /// Starts an instance with horrible passwords values.
    /// Obviously not to be emulated in production.
    ///
    /// Defaults to:
    /// - Admin: (username: `"admin"`, password: `"adminpassword"`, dn: `"cn=admin,dc=example,dc=org"`)
    /// - Users: `[]`
    /// - Accesslog admin: `None`
    /// - Anonymous bindings: `true`
    fn default() -> Self {
        Self {
            users: vec![],
            env_vars: HashMap::new(),
        }
    }
}

impl Image for OpenLDAP {
    fn name(&self) -> &str {
        NAME
    }

    fn tag(&self) -> &str {
        TAG
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        // maybe OpenLDAP will have a healthcheck someday
        // https://github.com/osixia/docker-openldap/issues/637
        vec![
            WaitFor::message_on_stderr("** Starting slapd **"),
            WaitFor::seconds(2), // ideally, one should wait for a port instead of waiting a fixed time
        ]
    }

    fn env_vars(
        &self,
    ) -> impl IntoIterator<Item = (impl Into<Cow<'_, str>>, impl Into<Cow<'_, str>>)> {
        let mut vars = self.env_vars.clone();
        let users = self
            .users
            .clone()
            .into_iter()
            .map(|u| u.username)
            .collect::<Vec<String>>();
        vars.insert("LDAP_USERS".to_owned(), users.join(", "));
        let passwords = self
            .users
            .clone()
            .into_iter()
            .map(|u| u.password)
            .collect::<Vec<String>>();
        vars.insert("LDAP_PASSWORDS".to_owned(), passwords.join(", "));
        vars
    }

    fn expose_ports(&self) -> &[ContainerPort] {
        &[OPENLDAP_PORT]
    }
}

#[cfg(test)]
mod tests {
    use ldap3::{Ldap, LdapConnAsync, LdapError, Scope, SearchEntry};
    use testcontainers::{runners::AsyncRunner, ImageExt};

    use super::*;

    async fn read_users(
        ldap: &mut Ldap,
        filter: &str,
        attrs: &[&str],
    ) -> Result<Vec<SearchEntry>, LdapError> {
        let (rs, _res) = ldap
            .search("ou=users,dc=example,dc=org", Scope::Subtree, filter, attrs)
            .await?
            .success()?;
        let res = rs.into_iter().map(SearchEntry::construct).collect();
        Ok(res)
    }

    async fn read_access_log(
        ldap: &mut Ldap,
        filter: &str,
        attrs: &[&str],
    ) -> Result<Vec<SearchEntry>, LdapError> {
        let (rs, _res) = ldap
            .search("cn=accesslog", Scope::Subtree, filter, attrs)
            .await?
            .success()?;
        let res = rs.into_iter().map(SearchEntry::construct).collect();
        Ok(res)
    }

    #[tokio::test]
    async fn ldap_users_noauth() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let openldap_image = OpenLDAP::default()
            .with_allow_anon_binding(true)
            .with_user("maximiliane", "pwd1")
            .with_user("maximus", "pwd2")
            .with_user("lea", "pwd3");
        let node = openldap_image.start().await?;

        let connection_string = format!(
            "ldap://{}:{}",
            node.get_host().await?,
            node.get_host_port_ipv4(OPENLDAP_PORT).await?,
        );
        let (conn, mut ldap) = LdapConnAsync::new(&connection_string).await?;
        ldap3::drive!(conn);
        let users = read_users(&mut ldap, "(cn=ma*)", &["cn"]).await?;
        let users: HashMap<String, _> = users.into_iter().map(|u| (u.dn, u.attrs)).collect();
        let expected_result_maximiliane = (
            "cn=maximiliane,ou=users,dc=example,dc=org".to_string(),
            HashMap::from([(
                "cn".to_string(),
                vec!["User1".to_string(), "maximiliane".to_string()],
            )]),
        );
        let expected_result_maximus = (
            "cn=maximus,ou=users,dc=example,dc=org".to_string(),
            HashMap::from([(
                "cn".to_string(),
                vec!["User2".to_string(), "maximus".to_string()],
            )]),
        );
        assert_eq!(
            users,
            HashMap::from([expected_result_maximus, expected_result_maximiliane])
        );
        ldap.unbind().await?;
        Ok(())
    }

    #[tokio::test]
    async fn ldap_users_simple_bind() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let openldap_image = OpenLDAP::default()
            .with_allow_anon_binding(false)
            .with_user("maximiliane", "pwd1");
        let node = openldap_image.start().await?;

        let connection_string = format!(
            "ldap://{}:{}",
            node.get_host().await?,
            node.get_host_port_ipv4(OPENLDAP_PORT).await?,
        );
        let (conn, mut ldap) = LdapConnAsync::new(&connection_string).await?;
        ldap3::drive!(conn);
        ldap.simple_bind("cn=maximiliane,ou=users,dc=example,dc=org", "pwd1")
            .await?
            .success()?;
        let users = read_users(&mut ldap, "(cn=*)", &["cn"]).await?;
        assert_eq!(users.len(), 2); // cn=maximiliane and cn=readers
        ldap.unbind().await?;
        Ok(())
    }

    #[tokio::test]
    async fn ldap_access_logs_noauth() -> Result<(), Box<dyn std::error::Error + 'static>> {
        let _ = pretty_env_logger::try_init();
        let openldap_image = OpenLDAP::default()
            .with_allow_anon_binding(true)
            .with_accesslog_settings(
                AccesslogSettings::default().with_log_operations(AccesslogLogOperations::Reads),
            );
        let node = openldap_image.start().await?;

        let connection_string = format!(
            "ldap://{}:{}",
            node.get_host().await?,
            node.get_host_port_ipv4(OPENLDAP_PORT).await?,
        );
        let (conn, mut ldap) = LdapConnAsync::new(&connection_string).await?;
        ldap3::drive!(conn);

        let access = read_access_log(&mut ldap, "(reqType=search)", &["*"]).await?;
        assert_eq!(access.len(), 0, "no search until now");

        let users = read_users(&mut ldap, "(cn=*)", &["cn"]).await?;
        assert_eq!(users.len(), 3, "cn=readers should be read");

        let access = read_access_log(&mut ldap, "(reqType=search)", &["*"]).await?;
        assert_eq!(access.len(), 1, "access log contains 1xread_users");

        ldap.unbind().await?;
        Ok(())
    }
}
