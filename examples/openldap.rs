use testcontainers::runners::SyncRunner;
use testcontainers_modules::openldap::OpenLDAP;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    // startup the module
    let node = OpenLDAP::default()
        .with_user("joe_average", "password")
        .start()?;

    // prepare connection string
    let connection_string = &format!("ldap://127.0.0.1:{}", node.get_host_port_ipv4(5432)?);
    // container is up, you can use it
    let mut con = ldap3::LdapConn::new(connection_string)?;
    let search_res = con.search(
        "ou=users,dc=example,dc=org",
        ldap3::Scope::Subtree,
        "(cn=*)",
        vec!["cn"],
    );
    assert_eq!(search_res.iter().len(), 1);
    Ok(())
}
