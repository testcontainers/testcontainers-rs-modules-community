use testcontainers::runners::SyncRunner;
use testcontainers_modules::anvil::{AnvilNode, ANVIL_PORT};

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    // Start the Anvil node
    let node = AnvilNode::default().start()?;

    // Get the mapped port for the Anvil JSON-RPC endpoint
    let port = node.get_host_port_ipv4(ANVIL_PORT)?;
    let rpc_url = format!("http://localhost:{port}");

    println!("Anvil node started successfully!");
    println!("JSON-RPC endpoint: {rpc_url}");
    println!();
    println!("You can now connect to this endpoint using your Ethereum tooling:");
    println!("  - cast:  cast block-number --rpc-url {rpc_url}");
    println!("  - web3:  new Web3(new Web3.providers.HttpProvider('{rpc_url}'))");
    println!("  - alloy: Provider::try_from('{rpc_url}')");
    println!();
    println!("Press Ctrl+C to stop the container");

    // Keep the container running
    std::thread::park();

    Ok(())
}
