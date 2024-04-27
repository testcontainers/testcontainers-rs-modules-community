use testcontainers::runners::SyncRunner;
use testcontainers_modules::postgres::Postgres;

fn main() {
    // startup the module
    let node = Postgres::default().start();

    // prepare connection string
    let connection_string = &format!(
        "postgres://postgres:postgres@127.0.0.1:{}/postgres",
        node.get_host_port_ipv4(5432)
    );
    // container is up, you can use it
    let mut conn = postgres::Client::connect(connection_string, postgres::NoTls).unwrap();
    let rows = conn.query("SELECT 1 + 1", &[]).unwrap();
    assert_eq!(rows.len(), 1);

    let first_row = &rows[0];
    let first_column: i32 = first_row.get(0);
    assert_eq!(first_column, 2);
}
